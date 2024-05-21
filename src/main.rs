// Save to json file
// Print current deltas, optional filter (regex match)
// New watch
// Start measure
// End measure

use std::fs::File;
use std::path::PathBuf;
use std::io::{self, Write};

use chrono::{self, DateTime, Datelike, Local, TimeZone, Timelike};
use clap::{Parser, Subcommand};
use dirs;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json;

const PATH: &str = "dotfiles/not_quite_dotfiles/watches";

fn main() {
    let args = Cli::parse();
    match args.command {
        Commands::New { name, movement } => handle_new(WatchBuilder{ name, movement }),
        Commands::Start { name }         => handle_start(name),
        Commands::End { name }           => panic!(),
        Commands::Ls { search }          => handle_search(search),
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
    let matches = get_matching_watches(&name);
    if matches.is_empty() {
        println!("No matches for regex [{}]", name);
        std::process::exit(1);
    }
    if matches.len() > 1 {
        println!("Multiple matches for regex [{}]:", name);
        println!("{:#?}", matches);
        std::process::exit(1);
    }

    let mut w = matches[0].clone();
    print!("Press [Enter] at watch's :00... ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input)
        .expect("Failed to read line");

    let now = Local::now();
    w.measure_start.real_time = now;
    let presumptive_watch_time = now.checked_add_signed(chrono::TimeDelta::seconds(15)).unwrap();
    println!("Assuming watch time of [{}]", presumptive_watch_time.format("%H:%M"));
    print!("[Enter] to accept, or give correction: ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input)
        .expect("Failed to read line");

    let watch_time = match input.trim() {
        "" => Local.with_ymd_and_hms(now.year(), now.month(), now.day(),
                presumptive_watch_time.hour(), presumptive_watch_time.minute(), 00),
        _ => {
            let re = Regex::new(r"(\d{2}):(\d{2})").unwrap();
            let caps = re.captures(input.trim()).unwrap();
            Local.with_ymd_and_hms(now.year(), now.month(), now.day(),
                caps[1].parse().unwrap(), caps[2].parse().unwrap(), 00)
        },
    }.unwrap();
    w.measure_start.watch_time = watch_time;
    w.save()
}
fn handle_search(query: String) {
    // TODO - spritz this up
    println!("{:#?}", get_matching_watches(&query));
}

fn start_watch_measure() {
    // Ask for enter key exactly at :00
    // Assume like +15 seconds, allow for correction
    // Take delta, save it off
}

fn update_watch_measure() {
    // Assume 0s drift, allow for correction
    // Compare to stored value
    // print + save
}

fn get_matching_watches(query: &str) -> Vec<Watch> {
    let re = Regex::new(&query).unwrap();
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
    println!("Loading path: {:?}", path);
    let file = File::open(&path).unwrap_or_else(|_| {
        println!("No file at path, creating [{:?}]", path);
        File::create(&path).expect(&format!("Can't create [{:?}]", path))
    });
    let reader = io::BufReader::new(file);
    let watches = serde_json::from_reader(reader).unwrap_or_else(|_| {
        println!("WARNING: file is empty, starting a new database");
        Vec::new()
    });
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
    measure_start: WatchTimePair,
    measure_end: WatchTimePair,
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
impl Default for WatchTimePair {
    fn default() -> Self {
        let t = Local.with_ymd_and_hms(2000, 2, 2, 2, 2, 2).unwrap();
        WatchTimePair {
            watch_time: t,
            real_time: t,
        }
    }
}
#[derive(Serialize, Deserialize, clap::ValueEnum, Clone, Debug)]
enum Movement {
    Quartz,
    Mechanical,
}

impl Watch {
    fn new() -> Self {
        Watch {
            name: String::new(),
            movement: Movement::Quartz,
            measure_start: WatchTimePair::default(),
            measure_end: WatchTimePair::default(),
        }
    }

    fn save(&self) {
        println!("Saving watch: {:#?}", self);
        let mut watches = load_file();
        let mut found = false;
        for w in &mut watches {
            if w.name == self.name {
                println!("Found watch with the same name, updating");
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
}

#[derive(Parser)]
#[command(name = "wd")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    /// Create a new watch
    New {
        /// Name of the watch
        #[clap(short)]
        name: Option<String>,
        /// quartz or mechanical movement
        #[clap(short, value_enum)]
        movement: Option<Movement>,
    },

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
}
