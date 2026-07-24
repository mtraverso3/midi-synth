mod audio;
mod midi;
mod output;
mod render;
mod sequencer;
mod synth;
mod tui;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use clap::Parser;

/// A MIDI file player with a built-in software synthesizer.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// MIDI file to play.
    #[arg(default_value = "assets/demo.mid")]
    file: PathBuf,

    /// Print each note event as it plays.
    #[arg(short, long)]
    verbose: bool,

    /// Show a live terminal visualizer while playing.
    #[arg(long)]
    tui: bool,

    /// Render to an audio file (.wav or .mp3) instead of playing live.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

const SAMPLE_RATE: u32 = 44_100;
const TAIL_SECONDS: f64 = 2.0;

fn main() {
    let cli = Cli::parse();

    let events = match midi::load(&cli.file) {
        Ok(events) => events,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    if let Some(path) = cli.output {
        println!("Loaded {} events from {}", events.len(), cli.file.display());
        println!("Rendering to {}...", path.display());
        let samples = render::render(&events, SAMPLE_RATE);
        if let Err(e) = output::write(&path, &samples, SAMPLE_RATE) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        println!("Wrote {}", path.display());
        return;
    }

    if cli.tui {
        let title = cli.file.file_name().and_then(|n| n.to_str()).unwrap_or("");
        play_with_tui(events, title.to_string());
        return;
    }

    println!("Loaded {} events from {}", events.len(), cli.file.display());
    let (_stream, tx) = audio::build_stream();
    println!("Playing...");
    sequencer::play(&events, &tx, cli.verbose, None);

    thread::sleep(Duration::from_millis(500));
    println!("Done.");
}

fn play_with_tui(events: Vec<midi::Event>, title: String) {
    let total_s = events.last().map(|e| e.time_s).unwrap_or(0.0) + TAIL_SECONDS;
    let monitor = Arc::new(Mutex::new(sequencer::Monitor::default()));

    let (_stream, tx) = audio::build_stream();

    let player_monitor = monitor.clone();
    let player = thread::spawn(move || {
        sequencer::play(&events, &tx, false, Some(&player_monitor));
    });

    if let Err(e) = tui::run(&monitor, &title, total_s) {
        eprintln!("tui error: {e}");
    }
    let _ = player.join();
}
