// Save to json file
// Print current deltas, optional filter (regex match)
// New watch
// Start measure
// End measure

use std::fs::File;
use std::time::Instant;
use std::io;

use clap::{Parser, Subcommand};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json;

const PATH: &str = "$HOME/dotfiles/not_quite_dotfiles/watches";

fn main() {
    let args = Cli::parse();

    // See if fits command list
    match args.command {
        Commands::New { name, movement } => handle_new(WatchBuilder{ name, movement }),
        Commands::Start { name } => panic!(),
        Commands::End { name } => panic!(),
        Commands::Ls { search } => handle_search(search),
    }
}
fn handle_new(wb: WatchBuilder) {
    let mut watch = Watch::new();

    watch.name = if wb.name.is_none() {
        String::from("TODO - get input")
    } else {
        wb.name.unwrap()
    };
    watch.movement = if wb.movement.is_none() {
        Movement::Quartz
    } else {
        wb.movement.unwrap()
    };

    watch.save();
}
fn handle_search(query: String) {
    let re = Regex::new(&query).unwrap();
    get_watches(re);
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

// fn get_sync_time() -> WatchTimePair {
//     // Ask for enter key exactly at :00
//     let mut s = String::new();
//     println!("Press [Enter] when the second hand hits exactly :00");
//     let timepair = match io::stdin().read_line(&mut s) {
//         Ok(_) => {
//             let now = Instant::now()
//         }
//         Err(e) => {
//             panic!("Got error when getting stdin: {}", e);
//         }
//     };
// }


fn get_watches(re: Regex) -> Vec<Watch> {
    let watches = load_file();

    // Filter with regex
    // return matching watches
    Vec::new()
}

fn load_file() -> Vec<Watch> {
    let file = File::open(PATH).unwrap();
    let reader = io::BufReader::new(file);
    let watches = serde_json::from_reader(reader).unwrap();
    watches
}
fn save_file(w: Vec<Watch>) {
    let file = File::open(PATH).unwrap();
    let writer = io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &w).unwrap();
}


#[derive(Serialize, Deserialize, Clone)]
struct Watch {
    name: String,
    movement: Movement,
    measure_running: bool,
    measure_start_diff: std::time::Duration,
    measure_end_diff: std::time::Duration,
}
struct WatchBuilder {
    name: Option<String>,
    movement: Option<Movement>,
}
#[derive(Serialize, Deserialize, clap::ValueEnum, Clone)]
enum Movement {
    Quartz,
    Mechanical,
}

impl Watch {
    fn new() -> Self {
        Watch {
            name: String::new(),
            movement: Movement::Quartz,
            measure_running: false,
            measure_start_diff: std::time::Duration::new(0, 0),
            measure_end_diff: std::time::Duration::new(0, 0),
        }
    }

    fn save(&self) {
        let mut watches = load_file();
        let mut found = false;
        for w in &mut watches {
            if w.name == self.name {
                found                = true;
                *w = self.clone();
                break;
            }
        }

        if !found {
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
        name: Option<String>,
        /// quartz or mechanical movement
        #[clap(value_enum)]
        movement: Option<Movement>,
    },

    Ls {
        /// Regex string used to filter watches
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
