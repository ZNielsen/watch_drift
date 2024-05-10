// Save to json file
// Print current deltas, optional filter (regex match)
// New watch
// Start measure
// End measure

use std::time::Instant;
use std::io;

fn main() {

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

fn get_sync_time() -> WatchTimePair {
    // Ask for enter key exactly at :00
    let mut s = String::new();
    println!("Press [Enter] when the second hand hits exactly :00");
    let timepair = match io::stdin().read_line(&mut s) {
        Ok(_) => {
            let now = Instant::now()
        }
        Err(e) => {
            panic!("Got error when getting stdin: {}", e);
        }
    };
}

struct Watch {
    name: String,
    movement: Movement,
    measure_start: WatchTimePair,
    measure_end: WatchTimePair,
}
enum Movement {
    Quartz,
    Mechanical,
}
struct WatchTimePair {
    watch_time: std::time::Instant,
    real_time: std::time::Instant,
}
impl Default for WatchTimePair {
    fn default() -> Self {
        let now = Instant::now();
        WatchTimePair {
            watch_time: now,
            real_time: now,
        }
    }
}

impl Watch {
    fn new() -> Self {
        // Ask for input
    }
    fn new_from_info(name: String, movement: Movement) -> Self {
        Watch {
            name,
            movement,
            measure_start: WatchTimePair::default(),
            measure_end: WatchTimePair::default()
        }
    }

    fn save(&self) -> Result<(), String> {
        // Load file
        // Look for this watch
        // If it's there, update the values, if not append
    }
    fn load(name: String) -> Result<Self, String> {
        // Load file
        // Look for this watch
        // Return it
    }
}
