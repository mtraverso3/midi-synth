use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::midi::{Event, EventKind};
use crate::synth::SynthCommand;

/// Live snapshot of what's playing, shared with the TUI. `active[ch]` is a
/// bitmask of sounding notes on that channel; `seen` marks channels that have
/// ever played so the UI only shows those.
#[derive(Default, Clone, Copy)]
pub struct Monitor {
    pub active: [u128; 16],
    pub programs: [u8; 16],
    pub seen: u16,
    pub notes_played: u64,
    pub finished: bool,
}

pub type SharedMonitor = Arc<Mutex<Monitor>>;

/// Plays events through `tx`, sleeping until each one's absolute timestamp so
/// scheduling error can't accumulate over the song. Assumes `events` is sorted
/// by `time_s`. Updates `monitor` (if given) as notes start and stop.
pub fn play(
    events: &[Event],
    tx: &Sender<SynthCommand>,
    verbose: bool,
    monitor: Option<&SharedMonitor>,
) {
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
        if let Some(monitor) = monitor {
            update_monitor(monitor, event);
        }

        if tx.send(to_command(event)).is_err() {
            break;
        }
    }

    if let Some(monitor) = monitor {
        monitor.lock().unwrap().finished = true;
    }
}

fn update_monitor(monitor: &SharedMonitor, event: &Event) {
    let mut m = monitor.lock().unwrap();
    let ch = event.channel as usize;
    m.seen |= 1 << ch;
    match event.kind {
        EventKind::NoteOn { note, .. } => {
            m.active[ch] |= 1 << note;
            m.notes_played += 1;
        }
        EventKind::NoteOff { note } => m.active[ch] &= !(1 << note),
        EventKind::ProgramChange { program } => m.programs[ch] = program,
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
