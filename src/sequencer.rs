use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::midi::{Event, EventKind};
use crate::synth::SynthCommand;

/// How often the play loop wakes to advance the song clock and fire due events.
const TICK: Duration = Duration::from_millis(4);

/// Seconds jumped by one seek step.
pub const SEEK_STEP: f64 = 5.0;

/// A transport command from the UI to the play loop.
pub enum Control {
    TogglePause,
    Seek(f64),
    Stop,
}

/// Live snapshot of what's playing, shared with the TUI. `active[ch]` is a
/// bitmask of sounding notes on that channel; `seen` marks channels that have
/// ever played so the UI only shows those.
#[derive(Default, Clone, Copy)]
pub struct Monitor {
    pub active: [u128; 16],
    pub programs: [u8; 16],
    pub seen: u16,
    pub notes_played: u64,
    pub song_time: f64,
    pub paused: bool,
    pub finished: bool,
}

pub type SharedMonitor = Arc<Mutex<Monitor>>;

/// Plays sorted `events` through `tx` on a song clock that advances in real
/// time, freezes while paused, and jumps on seek — driven by `controls`.
/// `total_s` (including the release tail) keeps the clock running past the end.
pub fn play(
    events: &[Event],
    tx: &Sender<SynthCommand>,
    total_s: f64,
    verbose: bool,
    monitor: Option<&SharedMonitor>,
    controls: &Receiver<Control>,
) {
    let mut song_time = 0.0f64;
    let mut next = 0usize; // index of the next event to fire
    let mut paused = false;
    let mut last = Instant::now();

    loop {
        for control in controls.try_iter() {
            match control {
                Control::Stop => return,
                Control::TogglePause => {
                    paused = !paused;
                    let _ = tx.send(SynthCommand::SetPaused(paused));
                }
                Control::Seek(delta) => {
                    song_time = (song_time + delta).clamp(0.0, total_s);
                    next = seek(events, tx, song_time, monitor);
                }
            }
        }

        let now = Instant::now();
        let dt = now.duration_since(last).as_secs_f64();
        last = now;
        if !paused {
            song_time += dt;
        }

        while next < events.len() && events[next].time_s <= song_time {
            let event = &events[next];
            if verbose {
                log_event(event, song_time);
            }
            if let Some(monitor) = monitor {
                update_monitor(monitor, event);
            }
            if tx.send(to_command(event)).is_err() {
                return;
            }
            next += 1;
        }

        if let Some(monitor) = monitor {
            let mut m = monitor.lock().unwrap();
            m.song_time = song_time;
            m.paused = paused;
        }

        if next >= events.len() && song_time >= total_s {
            break;
        }
        sleep(TICK);
    }

    if let Some(monitor) = monitor {
        monitor.lock().unwrap().finished = true;
    }
}

/// Jump the synth to `song_time`: silence held notes, then rebuild per-channel
/// state (program, volume, sustain) from the events before the target and
/// rewind the event cursor.
fn seek(
    events: &[Event],
    tx: &Sender<SynthCommand>,
    song_time: f64,
    monitor: Option<&SharedMonitor>,
) -> usize {
    let _ = tx.send(SynthCommand::AllNotesOff);

    let index = events.partition_point(|e| e.time_s < song_time);
    for event in &events[..index] {
        if matches!(
            event.kind,
            EventKind::ProgramChange { .. } | EventKind::Volume { .. } | EventKind::Sustain { .. }
        ) {
            let _ = tx.send(to_command(event));
        }
    }

    if let Some(monitor) = monitor {
        let mut m = monitor.lock().unwrap();
        m.active = [0; 16];
        m.song_time = song_time;
    }
    index
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
        EventKind::Sustain { .. } | EventKind::Volume { .. } => {}
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
        EventKind::Sustain { on } => SynthCommand::Sustain {
            channel: event.channel,
            on,
        },
        EventKind::Volume { level } => SynthCommand::SetVolume {
            channel: event.channel,
            level,
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
        EventKind::Sustain { on } => {
            println!(
                "[{now:7.3}s] ch{ch:2} sustain   {}",
                if on { "on" } else { "off" }
            )
        }
        EventKind::Volume { level } => println!("[{now:7.3}s] ch{ch:2} volume    {level}"),
    }
}

fn note_name(note: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = note as i32 / 12 - 1;
    format!("{}{}", NAMES[note as usize % 12], octave)
}
