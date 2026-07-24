use std::path::Path;

use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

#[derive(Debug, Clone, Copy)]
pub enum NoteEventKind {
    On { velocity: u8 },
    Off,
}

#[derive(Debug, Clone, Copy)]
pub struct NoteEvent {
    pub time_s: f64,
    pub note: u8,
    pub kind: NoteEventKind,
}

const DEFAULT_TEMPO_US_PER_BEAT: u32 = 500_000;

pub fn load(path: impl AsRef<Path>) -> Result<Vec<NoteEvent>, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    let smf = Smf::parse(&bytes)?;

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

        match kind {
            TrackEventKind::Meta(MetaMessage::Tempo(t)) => us_per_beat = t.as_int() as f64,
            TrackEventKind::Midi { message, .. } => match message {
                MidiMessage::NoteOn { key, vel } if vel.as_int() == 0 => events.push(NoteEvent {
                    time_s: current_seconds,
                    note: key.as_int(),
                    kind: NoteEventKind::Off,
                }),
                MidiMessage::NoteOn { key, vel } => events.push(NoteEvent {
                    time_s: current_seconds,
                    note: key.as_int(),
                    kind: NoteEventKind::On {
                        velocity: vel.as_int(),
                    },
                }),
                MidiMessage::NoteOff { key, .. } => events.push(NoteEvent {
                    time_s: current_seconds,
                    note: key.as_int(),
                    kind: NoteEventKind::Off,
                }),
                _ => {}
            },
            _ => {}
        }
    }

    Ok(events)
}
