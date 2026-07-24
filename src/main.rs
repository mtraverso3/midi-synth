mod audio;
mod midi;
mod output;
mod render;
mod sequencer;
mod synth;
mod tui;

use std::path::PathBuf;
use std::sync::mpsc;
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

/// Silence rendered after the last event so release tails aren't clipped.
pub(crate) const TAIL_SECONDS: f64 = 2.0;

fn main() {
    let cli = Cli::parse();

    let events = match midi::load(&cli.file) {
        Ok(events) => events,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    println!("Loaded {} events from {}", events.len(), cli.file.display());

    if let Some(path) = cli.output {
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

    let total_s = total_seconds(&events);
    let (_stream, tx) = audio::build_stream();
    let (_ctl_tx, ctl_rx) = mpsc::channel();
    println!("Playing...");
    sequencer::play(&events, &tx, total_s, cli.verbose, None, &ctl_rx);

    thread::sleep(Duration::from_millis(500));
    println!("Done.");
}

fn total_seconds(events: &[midi::Event]) -> f64 {
    events.last().map(|e| e.time_s).unwrap_or(0.0) + TAIL_SECONDS
}

fn play_with_tui(events: Vec<midi::Event>, title: String) {
    let total_s = total_seconds(&events);
    let monitor = Arc::new(Mutex::new(sequencer::Monitor::default()));
    let (ctl_tx, ctl_rx) = mpsc::channel();

    let (_stream, tx) = audio::build_stream();

    let player_monitor = monitor.clone();
    let player = thread::spawn(move || {
        sequencer::play(&events, &tx, total_s, false, Some(&player_monitor), &ctl_rx);
    });

    if let Err(e) = tui::run(&monitor, &ctl_tx, &title, total_s) {
        eprintln!("tui error: {e}");
    }
    let _ = player.join();
}
