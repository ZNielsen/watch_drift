// © Zach Nielsen 2024

use std::fs::File;
use std::path::PathBuf;
use std::io::{self, Write};

use crossterm::{self, event::KeyCode};
use chrono::{self, DateTime, Datelike, Local, naive::NaiveDate, TimeZone, Timelike};
use clap::{Parser, Subcommand};
use dirs;
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use serde_json;

const PATH: &str = "dotfiles/not_quite_dotfiles/watches";

fn main() {
    let args = Cli::parse();
    match args.command {
        Commands::New { name, movement } => handle_new(WatchBuilder{ name, movement }),
        Commands::Start { name }         => handle_start(name),
        Commands::End { name }           => handle_end(name),
        Commands::Ls { search }          => handle_search(search),
        Commands::Recalculate { search } => handle_recalculate(search),
        Commands::Log { name }           => handle_log(name),
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
    let now = get_00_time();
    let watch_time = get_watch_time_from_real_time(now);

    w.measure_start = Some( WatchTimePair {
        real_time: now,
        watch_time,
    });
    w.measure_end = None;
    w.save()
}
fn handle_end(name: String) {
    let mut w = get_matching_watch(name);
    println!("Ending measure for [{}]", w.name);
    let now = get_00_time();
    let watch_time = get_watch_time_from_real_time(now);

    w.measure_end = Some( WatchTimePair {
        real_time: now,
        watch_time,
    });
    w.update_running();
    w.save();

    let start = w.measure_start.unwrap();
    let end = w.measure_end.unwrap();
    let s = end.real_time.signed_duration_since(start.real_time).num_seconds();
    let hectodays  = s as f64 / 864.0;
    let days = hectodays.round() / 100.0;

    println!("\n");
    println!("Watch is running at {:+} seconds per {}, measured over {} days",
        w.running.unwrap(), w.movement.unit_str(), days);
    println!("")
}
fn handle_search(query: String) {
    let watches = get_matching_watches(&query);
    for w in watches {
        println!("Name: {}", w.name);
        println!("  Movement: {}", w.movement.to_str());

        if let Some(run) = w.running {
            println!("  Running at: {:+} seconds per {}", run, w.movement.unit_str());
        } else {
            println!("  No measure yet");
        }

        // 'measured over'
        if let Some(end) = w.measure_end {
            let start = w.measure_start.unwrap();
            let s = end.real_time.signed_duration_since(start.real_time).num_seconds();
            let hectodays  = s as f64 / 864.0;
            println!("  Measured over: {} days", hectodays.round() / 100.00);
        }
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
        println!("Already logged watch for today, not adding again.");
    }
}

fn get_matching_watch(query: String) -> Watch {
    let matches = get_matching_watches(&query);
    if matches.is_empty() {
        println!("No matches for regex [{}]", query);
        std::process::exit(1);
    }
    if matches.len() > 1 {
        println!("Multiple matches for regex [{}]:", query);
        println!("{:#?}", matches);
        std::process::exit(1);
    }
    matches[0].clone()
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
    let mut watch_time = t.checked_add_signed(chrono::TimeDelta::seconds(20)).unwrap();
    print!("Enter watch time, adjust with ↑/↓: [{}]", watch_time.format("%H:%M"));
    // print!("Enter watch time, adjust with [Up]/[Down]: [{}]", watch_time.format("%H:%M"));
    stdout.flush().unwrap();

    let (_cursor_x, cursor_y) = crossterm::cursor::position().unwrap();

    let mut update_time = |time: &DateTime<Local>| {
        // 38/42
        crossterm::execute!(stdout, crossterm::cursor::MoveTo(35, cursor_y)).unwrap();
        crossterm::execute!(stdout, crossterm::terminal::Clear(crossterm::terminal::ClearType::UntilNewLine)).unwrap();
        print!("[{}]", time.format("%H:%M"));
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
    println!("");

    crossterm::terminal::disable_raw_mode().unwrap();

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
    #[serde(skip_serializing_if = "Option::is_none")]
    measure_start: Option<WatchTimePair>,
    #[serde(skip_serializing_if = "Option::is_none")]
    measure_end: Option<WatchTimePair>,
    #[serde(skip_serializing_if = "Option::is_none")]
    running: Option<f64>,
    logs: Vec<NaiveDate>,
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
            measure_start: None,
            measure_end: None,
            running: None,
            logs: Vec::new(),
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

    fn update_running(&mut self) {
        let real_time_start  = self.measure_start.clone().unwrap().real_time;
        let watch_time_start = self.measure_start.clone().unwrap().watch_time;
        let real_time_end    = self.measure_end.clone().unwrap().real_time;
        let watch_time_end   = self.measure_end.clone().unwrap().watch_time;

        let real_time_passed = real_time_end.signed_duration_since(real_time_start);
        let watch_time_passed = watch_time_end.signed_duration_since(watch_time_start);
        let duration_diff = watch_time_passed.num_milliseconds() - real_time_passed.num_milliseconds();
        let diff_per_unit = (duration_diff * self.movement.unit()) as f64 / real_time_passed.num_milliseconds() as f64;
        self.running = Some(diff_per_unit.round() / 1000.0);
    }
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
        #[clap(default_value = "")]
        search: String,
    },

    /// Start a measure for the given watch
    Start {
        /// Name of the watch
        name: String,
    },

    /// End or Update a measure for the given watch
    End {
        /// Name of the watch
        name: String,
    },

    /// Force a recalculation of how the watch is running. Useful after manually editing the database file.
    Recalculate {
        #[clap(default_value = "")]
        search: String,
    },

    /// Mark down a wear for today of the given watch
    Log {
        name: String,
    },
}
