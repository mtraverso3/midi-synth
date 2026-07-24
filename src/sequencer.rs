use std::sync::mpsc::Sender;
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::midi::{Event, EventKind};
use crate::synth::SynthCommand;

/// Plays events through `tx`, sleeping until each one's absolute timestamp so
/// scheduling error can't accumulate over the song. Assumes `events` is sorted
/// by `time_s`.
pub fn play(events: &[Event], tx: &Sender<SynthCommand>, verbose: bool) {
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

        if tx.send(to_command(event)).is_err() {
            return;
        }
    }
}

pub fn to_command(event: &Event) -> SynthCommand {
    match event.kind {
        EventKind::NoteOn { note, velocity } => SynthCommand::NoteOn {
            channel: event.channel,
            note,
            velocity,
        },
        EventKind::NoteOff { note } => SynthCommand::NoteOff {
            channel: event.channel,
            note,
        },
        EventKind::ProgramChange { program } => SynthCommand::ProgramChange {
            channel: event.channel,
            program,
        },
    }
}

fn log_event(event: &Event, now: f64) {
    let ch = event.channel;
    match event.kind {
        EventKind::NoteOn { note, velocity } => println!(
            "[{now:7.3}s] ch{ch:2} NOTE ON   {:<4} (#{note:3})  vel {velocity}",
            note_name(note),
        ),
        EventKind::NoteOff { note } => println!(
            "[{now:7.3}s] ch{ch:2} note off  {:<4} (#{note:3})",
            note_name(note),
        ),
        EventKind::ProgramChange { program } => {
            println!("[{now:7.3}s] ch{ch:2} program   {program}")
        }
    }
}

fn note_name(note: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = note as i32 / 12 - 1;
    format!("{}{}", NAMES[note as usize % 12], octave)
}
