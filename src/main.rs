mod audio;
mod midi;
mod output;
mod render;
mod sequencer;
mod synth;

use std::path::PathBuf;
use std::thread::sleep;
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

    /// Render to an audio file (.wav or .mp3) instead of playing live.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

const SAMPLE_RATE: u32 = 44_100;

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

    let (_stream, tx) = audio::build_stream();
    println!("Playing...");
    sequencer::play(&events, &tx, cli.verbose);

    sleep(Duration::from_millis(500));
    println!("Done.");
}
