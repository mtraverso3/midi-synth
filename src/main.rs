mod audio;
mod output;
mod tui;

use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use clap::Parser;

use midi::sequencer::Monitor;
use midi::soundfont::SoundFont;
use midi::{TAIL_SECONDS, engine, render, sequencer, smf};

/// A MIDI file player with a built-in software synthesizer.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// MIDI file to play.
    #[arg(default_value = "assets/demo.mid")]
    file: PathBuf,

    /// Play through a SoundFont (.sf2) instead of the built-in synth.
    #[arg(short, long)]
    soundfont: Option<PathBuf>,

    /// Print each note event as it plays.
    #[arg(short, long)]
    verbose: bool,

    /// Show a live terminal visualizer while playing.
    #[arg(long)]
    tui: bool,

    /// Render to an audio file (.wav or .mp3) instead of playing live.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output gain in decibels, before the limiter.
    #[arg(short, long, default_value_t = 0.0, allow_negative_numbers = true)]
    gain: f32,
}

const SAMPLE_RATE: u32 = 44_100;

fn main() {
    let cli = Cli::parse();

    let events = or_exit(smf::load(&cli.file));
    println!("Loaded {} events from {}", events.len(), cli.file.display());

    let soundfont = cli.soundfont.as_ref().map(|path| {
        let bank = or_exit(SoundFont::load(path));
        println!("Using soundfont {}", path.display());
        bank
    });

    let gain = decibels(cli.gain);

    if let Some(path) = cli.output {
        println!("Rendering to {}...", path.display());
        let engine = or_exit(engine::build(soundfont, SAMPLE_RATE, gain));
        let samples = render::render(&events, SAMPLE_RATE, engine);
        if let Err(e) = output::write(&path, &samples, SAMPLE_RATE) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        println!("Wrote {}", path.display());
        return;
    }

    if cli.tui {
        let title = cli.file.file_name().and_then(|n| n.to_str()).unwrap_or("");
        play_with_tui(events, soundfont, title.to_string(), gain);
        return;
    }

    let total_s = total_seconds(&events);
    let (_stream, tx) = or_exit(audio::build_stream(soundfont, gain));
    let (_ctl_tx, ctl_rx) = mpsc::channel();
    println!("Playing...");
    sequencer::play(&events, &tx, total_s, cli.verbose, None, &ctl_rx);

    thread::sleep(Duration::from_millis(500));
    println!("Done.");
}

fn or_exit<T>(result: Result<T, Box<dyn std::error::Error>>) -> T {
    result.unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    })
}

fn decibels(db: f32) -> f32 {
    10.0f32.powf(db / 20.0)
}

fn total_seconds(events: &[smf::Event]) -> f64 {
    events.last().map(|e| e.time_s).unwrap_or(0.0) + TAIL_SECONDS
}

fn play_with_tui(events: Vec<smf::Event>, soundfont: Option<SoundFont>, title: String, gain: f32) {
    let total_s = total_seconds(&events);
    let monitor = Arc::new(Mutex::new(Monitor::default()));
    let (ctl_tx, ctl_rx) = mpsc::channel();

    let (_stream, tx) = or_exit(audio::build_stream(soundfont, gain));

    let player_monitor = monitor.clone();
    let player = thread::spawn(move || {
        sequencer::play(&events, &tx, total_s, false, Some(&player_monitor), &ctl_rx);
    });

    if let Err(e) = tui::run(&monitor, &ctl_tx, &title, total_s) {
        eprintln!("tui error: {e}");
    }
    let _ = player.join();
}
