// © Zach Nielsen 2024

use std::fs::File;
use std::path::PathBuf;
use std::io::{self, Write};
use std::cmp::max;

use chrono::{self, DateTime, Datelike, Local, naive::NaiveDate, TimeZone, Timelike};
use clap::{Parser, Subcommand};
use crossterm::{self, event::KeyCode};
use dirs;
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use serde_json;

const PATH: &str = "dotfiles/not_quite_dotfiles/watches";

fn main() {
    let args = Cli::parse();
    match args.command {
        Commands::New { name, movement } => handle_new(WatchBuilder{ name, movement }),
        Commands::Start { name }         => handle_start(name.join(" ")),
        Commands::End { name }           => handle_end(name.join(" ")),
        Commands::Ls { search }          => handle_ls(search.join(" ")),
        Commands::Recalculate { search } => handle_recalculate(search.join(" ")),
        Commands::Log { name }           => handle_log(name.join(" ")),
        Commands::Print { }              => handle_print(),
    }
}
fn handle_new(wb: WatchBuilder) {
    let mut watch = Watch::new();

    watch.name = if wb.name.is_none() {
        print!("Watch Name: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input)
            .expect("Failed to read line");
        input.trim().to_owned()
    } else {
        wb.name.unwrap()
    };
    watch.movement = if wb.movement.is_none() {
        let mut mvt = None;
        while mvt.is_none() {
            println!("Watch type");
            println!("  [1]: Quartz");
            println!("  [2]: Mechanical");
            print!  ("Enter (1/2): ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input)
                .expect("Failed to read line");
            mvt = match input.trim() {
                "1" => Some(Movement::Quartz),
                "2" => Some(Movement::Mechanical),
                _ => None,
            };
        }
        mvt.unwrap()
    } else {
        wb.movement.unwrap()
    };

    watch.save();
}
fn handle_start(name: String) {
    let mut w = get_matching_watch(name);
    println!("Starting measure for [{}]", w.name);
    if let Some(start) = w.measure_start() {
        println!("Overwriting start time: {:?}", start);
    }
    let now = get_00_time();
    let watch_time = get_watch_time_from_real_time(now);

    w.measures.push(Measure {
        measure_start: Some( WatchTimePair {
            real_time: now,
            watch_time,
        }),
        measure_end: None,
        drift: None,
    });
    w.save()
}
fn handle_end(name: String) {
    let mut w = get_matching_watch(name);
    if w.measures.last().unwrap().measure_end.is_some() {
        println!("End measure update for [{}]", w.name);
        println!("Updating measure:\n{}", w.measures.last().unwrap());
    } else {
        println!("Ending measure for [{}]", w.name);
    }
    let now = get_00_time();
    let watch_time = get_watch_time_from_real_time(now);

    w.measures.last_mut().unwrap().measure_end = Some( WatchTimePair {
        real_time: now,
        watch_time,
    });
    w.update_running();
    w.save();

    let (unit, units) = w.last_complete_measure().unwrap().get_measure_time();

    println!("\n");
    println!("Watch is running at {:+} seconds per {}, measured over {} {}",
        w.drift().unwrap(), w.movement.unit_str(), unit, units);
    println!("")
}
fn handle_ls(query: String) {
    let watches = get_matching_watches(&query);
    for w in watches {
        println!("Name: {}", w.name);
        println!("  Movement: {}", w.movement.to_str());

        if let Some(m) = w.last_complete_measure() {
            println!("  Running at: {:+} seconds per {}", m.drift.unwrap(), w.movement.unit_str());
            let (unit, units) = m.get_measure_time();
            println!("  Measured over: {} {}", unit, units);
        } else {
            println!("  No measure yet");
        }
        println!("  Worn on {} days", w.logs.len());

        println!("");
    }
}
fn handle_recalculate(query: String) {
    let mut watches= get_matching_watches(&query);
    for w in &mut watches {
        w.update_running();
        w.save();
    }
}
fn handle_log(name: String) {
    let mut w = get_matching_watch(name);
    println!("Tracking log for [{}]", w.name);
    print!("Confirm? [Enter], ^C to cancel: ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input)
        .expect("Failed to read line");

    let date = Local::now().date_naive();
    if !w.logs.contains(&date) {
        w.logs.push(date);
        w.save();
        println!("Added log for watch. Now worn on {} days.", w.logs.len())
    } else {
        println!("Already logged watch for today, not adding again (worn on {} days)", w.logs.len());
    }
}
fn handle_print() {
    let watches = get_matching_watches("");
    print_markdown_table(watches);
}

///////////////////////////////////////////////////////////////////////////////

fn get_matching_watch(query: String) -> Watch {
    let matches = get_matching_watches(&query);
    if matches.is_empty() {
        println!("No matches for regex [{}]", query);
        std::process::exit(1);
    }
    if matches.len() > 1 {
        println!("Multiple matches for regex [{}]:\n", query);
        return get_one_watch_from_matches(matches);
    }
    matches[0].clone()
}
fn get_one_watch_from_matches(watches: Vec<Watch>) -> Watch {
    println!("Choose with arrow keys:");
    for (idx, watch) in watches.iter().enumerate() {
        println!("[{}] {}", idx, watch.name);
    }

    let mut stdout = io::stdout();
    let (_cursor_x, cursor_y) = crossterm::cursor::position().unwrap();
    let y_offset = cursor_y - watches.len() as u16;
    let mut cursor_idx = 0;
    crossterm::execute!(stdout, crossterm::cursor::MoveTo(0, cursor_idx as u16 + y_offset)).unwrap();

    // closure: Clear and redraw arrows
    let mut update_selection = |cursor_idx: usize| {
        let (_, pre_move_y) = crossterm::cursor::position().unwrap();
        let pre_idx = match pre_move_y < cursor_idx as u16 + y_offset {
            true  => cursor_idx - 1,
            false => cursor_idx + 1,
        };
        crossterm::execute!(stdout,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
            crossterm::cursor::MoveTo(0, pre_move_y),
            crossterm::style::Print(format!("[{}] {}", pre_idx, watches[pre_idx].name)),
            crossterm::cursor::MoveTo(0, cursor_idx as u16 + y_offset),
            crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
            crossterm::style::Print(format!("[{}] --> {} <--", cursor_idx, watches[cursor_idx].name)),
        ).unwrap();
        stdout.flush().unwrap();
    };

    update_selection(cursor_idx);
    crossterm::terminal::enable_raw_mode().unwrap();
    loop {
        if crossterm::event::poll(std::time::Duration::from_millis(150)).unwrap() {
            if let crossterm::event::Event::Key(key_event) = crossterm::event::read().unwrap() {
                match key_event.code {
                    KeyCode::Up => {
                        if cursor_idx > 0 {
                            cursor_idx -= 1;
                            update_selection(cursor_idx);
                        }
                    },
                    KeyCode::Down => {
                        if cursor_idx < watches.len()-1 {
                            cursor_idx += 1;
                            update_selection(cursor_idx);
                        }
                    },
                    KeyCode::Char('c') => {
                        if key_event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                            crossterm::terminal::disable_raw_mode().unwrap();
                            print!("{}", "\n".repeat(1 + watches.len() - cursor_idx));
                            panic!("Got CTRL-C, hard quitting");
                        }
                    },
                    KeyCode::Enter => break,
                    _ => {},
                }
            }
        }
    }
    crossterm::terminal::disable_raw_mode().unwrap();
    // Push the printed watches out of the way of the to-come confirmation dialog
    print!("{}", "\n".repeat(1 + watches.len() - cursor_idx));
    watches[cursor_idx].clone()
}

fn get_00_time() -> DateTime<Local> {
    print!("Press [Enter] at watch's :00... ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input)
        .expect("Failed to read line");

    Local::now()
}

fn get_watch_time_from_real_time(t: DateTime<Local>) -> DateTime<Local> {
    let mut stdout = io::stdout();
    let mut watch_time = t.checked_add_signed(chrono::TimeDelta::seconds(55)).unwrap();
    print!("Enter watch time, adjust with ↑/↓: [{}]", watch_time.format("%H:%M"));
    // print!("Enter watch time, adjust with [Up]/[Down]: [{}]", watch_time.format("%H:%M"));
    stdout.flush().unwrap();

    let (_cursor_x, cursor_y) = crossterm::cursor::position().unwrap();

    // closure: Redraw time
    let mut update_time = |time: &DateTime<Local>| {
        // 35/38/42?
        crossterm::execute!(stdout,
            crossterm::cursor::MoveTo(35, cursor_y),
            crossterm::terminal::Clear(crossterm::terminal::ClearType::UntilNewLine),
            crossterm::style::Print(format!("[{}]", time.format("%H:%M"))),
        ).unwrap();
        stdout.flush().unwrap();
    };

    crossterm::terminal::enable_raw_mode().unwrap();
    loop {
        if crossterm::event::poll(std::time::Duration::from_millis(150)).unwrap() {
            if let crossterm::event::Event::Key(key_event) = crossterm::event::read().unwrap() {
                match key_event.code {
                    KeyCode::Up => {
                        watch_time = watch_time.checked_add_signed(chrono::TimeDelta::minutes(1)).unwrap();
                        update_time(&watch_time);
                    },
                    KeyCode::Down => {
                        watch_time = watch_time.checked_sub_signed(chrono::TimeDelta::minutes(1)).unwrap();
                        update_time(&watch_time);
                    },
                    KeyCode::Enter => break,
                    _ => {},
                }
            }
        }
    }
    crossterm::terminal::disable_raw_mode().unwrap();

    println!("\n");

    Local.with_ymd_and_hms(t.year(), t.month(), t.day(),
            watch_time.hour(), watch_time.minute(), 00).unwrap()
}

fn get_matching_watches(query: &str) -> Vec<Watch> {
    let re = RegexBuilder::new(&query)
        .case_insensitive(true)
        .build()
        .unwrap();
    let watches = load_file();
    let mut matching = Vec::new();
    for w in watches {
        if re.is_match(&w.name) {
            matching.push(w.clone())
        }
    }
    matching
}
fn get_path() -> PathBuf {
   let mut home_dir = match dirs::home_dir() {
        Some(path) => path,
        None => {
            println!("Couldn't determine the home directory.");
            panic!();
        }
    };
    home_dir.push(PATH);
    home_dir
}
fn load_file() -> Vec<Watch> {
    let path = get_path();
    let file = File::open(&path).unwrap_or_else(|_| {
        println!("No file at path, creating [{:?}]", path);
        File::create(&path).expect(&format!("Can't create [{:?}]", path))
    });
    let reader = io::BufReader::new(file);
    let watches = serde_json::from_reader(reader).unwrap();
    watches
}
fn save_file(w: Vec<Watch>) {
    let path = get_path();
    let file = File::create(&path).unwrap();
    let writer = io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &w).unwrap();
}


#[derive(Serialize, Deserialize, Clone, Debug)]
struct Watch {
    name: String,
    movement: Movement,
    logs: Vec<NaiveDate>,
    measures: Vec<Measure>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Measure {
    #[serde(skip_serializing_if = "Option::is_none")]
    drift: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    measure_start: Option<WatchTimePair>,
    #[serde(skip_serializing_if = "Option::is_none")]
    measure_end: Option<WatchTimePair>,
}
struct WatchBuilder {
    name: Option<String>,
    movement: Option<Movement>,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
struct WatchTimePair {
    watch_time: DateTime<Local>,
    real_time: DateTime<Local>,
}
#[derive(Serialize, Deserialize, clap::ValueEnum, Clone, Debug)]
enum Movement {
    Quartz,
    Mechanical,
}
impl Movement {
    fn unit(&self) -> i64 {
        match self {
            Movement::Quartz => 2628000000,
            Movement::Mechanical => 86400000,
        }
    }
    fn unit_str(&self) -> &str {
        match self {
            Movement::Quartz => "month",
            Movement::Mechanical => "day",
        }
    }
    fn to_str(&self) -> &str {
        match self {
            Movement::Quartz => "Quartz",
            Movement::Mechanical => "Mechanical",
        }
    }
}

impl Watch {
    fn new() -> Self {
        Watch {
            name: String::new(),
            movement: Movement::Quartz,
            logs: Vec::new(),
            measures: Vec::new(),
        }
    }

    fn save(&self) {
        println!("Saving watch: {:#?}", self);
        let mut watches = load_file();
        let mut found = false;
        for w in &mut watches {
            if w.name == self.name {
                found = true;
                *w = self.clone();
                break;
            }
        }
        if !found {
            println!("Adding new watch entry");
            watches.push(self.clone());
        }

        save_file(watches)
    }

    fn measure_start(&self) -> Option<WatchTimePair> {
        if let Some(m) = self.measures.last() {
            return m.measure_start.clone();
        }
        None
    }
    fn drift(&self) -> Option<f64> {
        if let Some(m) = self.measures.last() {
            return m.drift.clone();
        }
        None
    }
    fn last_complete_measure(&self) -> Option<&Measure> {
        for m in self.measures.iter().rev() {
            if m.measure_start.is_some() &&
               m.measure_end.is_some() &&
               m.drift.is_some() {
                   return Some(m);
            }
        }
        None
    }

    fn update_running(&mut self) {
        for m in &mut self.measures {
            if m.measure_start.is_none() || m.measure_end.is_none() {
                continue;
            }
            let real_time_start  = m.measure_start.as_ref().unwrap().real_time;
            let watch_time_start = m.measure_start.as_ref().unwrap().watch_time;
            let real_time_end    = m.measure_end.as_ref().unwrap().real_time;
            let watch_time_end   = m.measure_end.as_ref().unwrap().watch_time;

            let real_time_passed = real_time_end.signed_duration_since(real_time_start);
            let watch_time_passed = watch_time_end.signed_duration_since(watch_time_start);
            let duration_diff = watch_time_passed.num_milliseconds() - real_time_passed.num_milliseconds();
            let diff_per_unit = (duration_diff * self.movement.unit()) as f64 / real_time_passed.num_milliseconds() as f64;
            m.drift = Some(diff_per_unit.round() / 1000.0);
        }
    }

    fn table_print_name(&self) -> String {
        if self.measures.len() == 0 {
            return self.name.clone()
        }

        // Indicate if there is an active measure for this watch
        if self.measures.last().unwrap().measure_end.is_none() {
            return format!("* {} *", self.name);
        }

        self.name.clone()
    }
}
impl Measure {
    fn get_measure_time(&self) -> (f64, String) {
        let start = self.measure_start.as_ref().unwrap();
        let end = self.measure_end.as_ref().unwrap();
        let s = end.real_time.signed_duration_since(start.real_time).num_seconds();
        let hectodays  = s as f64 / 864.0;
        let mut unit = hectodays.round() / 100.0;
        let mut units = "days";
        if unit < 1.0 {
            // Do hours if less than 1 day
            let hectohours = s as f64 / 36.0;
            unit = hectohours.round() / 100.0;
            units = "hours";
        }

        (unit, units.to_owned())
    }
}
impl std::fmt::Display for Measure {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.drift.is_none() {
            write!(f, "  Drift: None\n")?;
        } else {
            write!(f, "  Drift: {:+} seconds\n", self.drift.as_ref().unwrap())?;
        }

        if self.measure_start.is_none() {
            write!(f, "  Start: None\n")?;
        } else {
            write!(f, "  Start:\n")?;
            write!(f, "    Watch: {}\n", self.measure_start.as_ref().unwrap().watch_time)?;
            write!(f, "    Real : {}\n", self.measure_start.as_ref().unwrap().real_time)?;
        }

        if self.measure_start.is_none() {
            write!(f, "  End: None\n")?;
        } else {
            write!(f, "  End:\n")?;
            write!(f, "    Watch: {}\n", self.measure_end.as_ref().unwrap().watch_time)?;
            write!(f, "    Real : {}\n", self.measure_end.as_ref().unwrap().real_time)?;
        }

        Ok(())
    }
}

fn print_markdown_table(mut watches: Vec<Watch>) {
    println!();
    // Watch Name | Type | Drift | # Wears
    let name_header = "Watch Name";
    let type_header = "Type";
    let drift_header = "Drift";
    let wears_header = "Num. Wears";

    // Sort by number of wears
    watches.sort_by_key(|w| w.logs.len());
    watches.reverse();

    // Get widths of the columns - Name
    // let max_name_len = 30;
    let mut name_len = name_header.len();
    let mut name_heights = Vec::new();
    for w in &watches {
        // TODO, GH-20: wrap names if they are too long
        // name_heights.push(1 + (w.table_print_name().len() / max_name_len));
        name_heights.push(1);
        name_len = max(name_len, w.table_print_name().len());
    }

    // Get widths of the columns - Type
    let type_len = max(type_header.len(), "Mechanical".len());

    // Get widths of the columns - Drift
    let mut drift_len = drift_header.len();
    for w in &watches {
        let drift_str = match w.drift() {
            Some(drift) => {
                let (unit, units) = w.last_complete_measure().unwrap().get_measure_time();
                format!("{:+}s/{}, ({} {})", drift, w.movement.unit_str(), unit, units)
            },
            None => "??".to_owned(),
        };
        drift_len = max(drift_len, drift_str.len());
    }

    // Get widths of the columns - Wears
    let mut wears_len = wears_header.len();
    for w in &watches {
        wears_len = max(wears_len, format!("{} days", w.logs.len()).len());
    }

    // Print the table
    // Header
    let (name_pad_l, name_pad_r) = get_left_right_padding(name_header, name_len);
    let (type_pad_l, type_pad_r) = get_left_right_padding(type_header, type_len);
    let (drift_pad_l, drift_pad_r) = get_left_right_padding(drift_header, drift_len);
    let (wears_pad_l, wears_pad_r) = get_left_right_padding(wears_header, wears_len);
    println!(
        "| {:n_l$}{n}{:n_r$} | {:t_l$}{t}{:t_r$} | {:d_l$}{d}{:d_r$} | {:w_l$}{w}{:w_r$} |", "", "", "", "", "", "", "", "",
        n_l = name_pad_l,
        n = name_header,
        n_r = name_pad_r,
        t_l = type_pad_l,
        t = type_header,
        t_r = type_pad_r,
        d_l = drift_pad_l,
        d = drift_header,
        d_r = drift_pad_r,
        w_l = wears_pad_l,
        w = wears_header,
        w_r = wears_pad_r,
    );
    println!("|{}|{}|{}|{}|",
        "-".repeat(name_len + 2),
        "-".repeat(type_len + 2),
        "-".repeat(drift_len + 2),
        "-".repeat(wears_len + 2),
    );

    // Looping over body
    for (watch, _) in watches.iter().zip(name_heights.iter()) {
        let n = &watch.table_print_name();
        let t = watch.movement.to_str();
        let d = match watch.drift() {
            Some(drift) => {
                let (unit, units) = watch.last_complete_measure().unwrap().get_measure_time();
                format!("{:+}s/{}, ({} {})", drift, watch.movement.unit_str(), unit, units)
            },
            None => "??".to_owned(),
        };
        let w = format!("{} days", watch.logs.len());

        let (name_pad_l, name_pad_r) = get_left_right_padding(&n, name_len);
        let (type_pad_l, type_pad_r) = get_left_right_padding(&t, type_len);
        let (drift_pad_l, drift_pad_r) = get_left_right_padding(&d, drift_len);
        let (wears_pad_l, wears_pad_r) = get_left_right_padding(&w, wears_len);
        println!(
            "| {:n_l$}{n}{:n_r$} | {:t_l$}{t}{:t_r$} | {:d_l$}{d}{:d_r$} | {:w_l$}{w}{:w_r$} |", "", "", "", "", "", "", "", "",
            n_l = name_pad_l,
            n_r = name_pad_r,
            t_l = type_pad_l,
            t_r = type_pad_r,
            d_l = drift_pad_l,
            d_r = drift_pad_r,
            w_l = wears_pad_l,
            w_r = wears_pad_r,
        );
    }
}

fn get_left_right_padding(s: &str, len: usize) -> (usize, usize) {
    let left = (len - s.len()) / 2;
    let right = len - s.len() - left;
    (left, right)
}

#[derive(Parser)]
#[command(name = "wd")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    /// Create a new watch. May pass in -n <name> and -m <movement>
    New {
        /// Name of the watch
        #[clap(short)]
        name: Option<String>,
        /// QUARTZ or MECHANICAL movement
        ///
        /// Used when calculating how the watch is running to give you Seconds per Day (spd) or
        /// Seconds per Month (spm)
        #[clap(short, value_enum)]
        movement: Option<Movement>,
    },

    /// Lists watches in the database. Takes an optional regex pattern to filter.
    Ls {
        /// Regex string used to filter watches
        #[clap(default_value = "", trailing_var_arg = true, allow_hyphen_values = true)]
        search: Vec<String>,
    },

    /// Start a measure for the given watch
    Start {
        /// Name of the watch
        #[clap(trailing_var_arg = true, allow_hyphen_values = true)]
        name: Vec<String>,
    },

    /// End or Update a measure for the given watch
    End {
        /// Name of the watch
        #[clap(trailing_var_arg = true, allow_hyphen_values = true)]
        name: Vec<String>,
    },

    /// Force a recalculation of how the watch is running. Useful after manually editing the database file.
    Recalculate {
        #[clap(default_value = "", trailing_var_arg = true, allow_hyphen_values = true)]
        search: Vec<String>,
    },

    /// Mark down a wear of the given watch for today
    Log {
        #[clap(trailing_var_arg = true, allow_hyphen_values = true)]
        name: Vec<String>,
    },

    /// Print all watches to a markdown table
    Print {
    },
}
