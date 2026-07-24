mod audio;
mod midi;
mod sequencer;
mod synth;

use std::thread::sleep;
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let path = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .cloned()
        .unwrap_or_else(|| "assets/twinkle.mid".to_string());

    let events = midi::load(&path).expect("failed to load MIDI file");
    println!("Loaded {} events from {path}", events.len());

    let (_stream, tx) = audio::build_stream();

    println!("Playing...");
    sequencer::play(&events, &tx, verbose);

    // Let the final release tail ring out before the stream is dropped.
    sleep(Duration::from_millis(500));
    println!("Done.");
}
