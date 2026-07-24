use std::sync::mpsc::Sender;
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::midi::{NoteEvent, NoteEventKind};
use crate::synth::SynthCommand;

/// Plays events through `tx`, sleeping until each one's absolute timestamp so
/// scheduling error can't accumulate over the song. Assumes `events` is sorted
/// by `time_s`.
pub fn play(events: &[NoteEvent], tx: &Sender<SynthCommand>, verbose: bool) {
    let start = Instant::now();

    for event in events {
        let target = Duration::from_secs_f64(event.time_s);
        let elapsed = start.elapsed();
        if target > elapsed {
            sleep(target - elapsed);
        }

        if verbose {
            log_event(event, start.elapsed().as_secs_f64());
        }

        let command = match event.kind {
            NoteEventKind::On { velocity } => SynthCommand::NoteOn {
                note: event.note,
                velocity,
            },
            NoteEventKind::Off => SynthCommand::NoteOff { note: event.note },
        };
        if tx.send(command).is_err() {
            return;
        }
    }
}

fn log_event(event: &NoteEvent, now: f64) {
    match event.kind {
        NoteEventKind::On { velocity } => println!(
            "[{now:7.3}s] NOTE ON   {:<4} (#{:3})  vel {velocity}",
            note_name(event.note),
            event.note,
        ),
        NoteEventKind::Off => println!(
            "[{now:7.3}s] note off  {:<4} (#{:3})",
            note_name(event.note),
            event.note,
        ),
    }
}

fn note_name(note: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = note as i32 / 12 - 1;
    format!("{}{}", NAMES[note as usize % 12], octave)
}
