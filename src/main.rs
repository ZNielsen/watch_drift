// Save to json file
// Print current deltas, optional filter (regex match)
// New watch
// Start measure
// End measure

use std::fs::File;
use std::path::PathBuf;
use std::io;

use clap::{Parser, Subcommand};
use dirs;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json;

const PATH: &str = "dotfiles/not_quite_dotfiles/watches";

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
        println!("Watch Name:");
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
            println!("Enter [1/2]:");
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
    measure_running: bool,
    measure_start_diff: std::time::Duration,
    measure_end_diff: std::time::Duration,
}
struct WatchBuilder {
    name: Option<String>,
    movement: Option<Movement>,
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
            measure_running: false,
            measure_start_diff: std::time::Duration::new(0, 0),
            measure_end_diff: std::time::Duration::new(0, 0),
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
