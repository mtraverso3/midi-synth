use std::path::Path;

use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

#[derive(Debug, Clone, Copy)]
pub enum EventKind {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
    ProgramChange { program: u8 },
    Sustain { on: bool },
    Volume { level: u8 },
}

const CC_VOLUME: u8 = 7;
const CC_SUSTAIN: u8 = 64;

#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub time_s: f64,
    pub channel: u8,
    pub kind: EventKind,
}

const DEFAULT_TEMPO_US_PER_BEAT: u32 = 500_000;

pub fn load(path: impl AsRef<Path>) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    parse(&bytes)
}

pub fn parse(bytes: &[u8]) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
    let smf = Smf::parse(bytes)?;

    let ticks_per_beat = match smf.header.timing {
        Timing::Metrical(t) => t.as_int() as f64,
        Timing::Timecode(_, _) => return Err("SMPTE timecode timing not supported".into()),
    };

    // Merge all tracks onto one timeline, resolving per-track delta-times into
    // absolute ticks.
    struct RawEvent<'a> {
        tick: u64,
        kind: TrackEventKind<'a>,
    }
    let mut raw = Vec::new();
    for track in &smf.tracks {
        let mut abs_tick = 0u64;
        for event in track {
            abs_tick += event.delta.as_int() as u64;
            raw.push(RawEvent {
                tick: abs_tick,
                kind: event.kind,
            });
        }
    }
    raw.sort_by_key(|e| e.tick);

    // Walk the timeline converting ticks to seconds, tracking tempo changes.
    let mut events = Vec::new();
    let mut current_seconds = 0.0f64;
    let mut last_tick = 0u64;
    let mut us_per_beat = DEFAULT_TEMPO_US_PER_BEAT as f64;

    for RawEvent { tick, kind } in raw {
        let delta_ticks = (tick - last_tick) as f64;
        let seconds_per_tick = (us_per_beat / 1_000_000.0) / ticks_per_beat;
        current_seconds += delta_ticks * seconds_per_tick;
        last_tick = tick;

        if let TrackEventKind::Meta(MetaMessage::Tempo(t)) = kind {
            us_per_beat = t.as_int() as f64;
            continue;
        }

        let TrackEventKind::Midi { channel, message } = kind else {
            continue;
        };
        let channel = channel.as_int();
        let kind = match message {
            MidiMessage::NoteOn { key, vel } if vel.as_int() == 0 => {
                EventKind::NoteOff { note: key.as_int() }
            }
            MidiMessage::NoteOn { key, vel } => EventKind::NoteOn {
                note: key.as_int(),
                velocity: vel.as_int(),
            },
            MidiMessage::NoteOff { key, .. } => EventKind::NoteOff { note: key.as_int() },
            MidiMessage::ProgramChange { program } => EventKind::ProgramChange {
                program: program.as_int(),
            },
            MidiMessage::Controller { controller, value } => match controller.as_int() {
                CC_SUSTAIN => EventKind::Sustain {
                    on: value.as_int() >= 64,
                },
                CC_VOLUME => EventKind::Volume {
                    level: value.as_int(),
                },
                _ => continue,
            },
            _ => continue,
        };
        events.push(Event {
            time_s: current_seconds,
            channel,
            kind,
        });
    }

    Ok(events)
}
