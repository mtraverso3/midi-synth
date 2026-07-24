use std::path::Path;

use midly::{Format, Fps, MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

/// Every MIDI 1.0 channel voice message, carried through as the file wrote it.
/// Interpreting controllers is the synth's job, not the parser's.
#[derive(Debug, Clone, Copy)]
pub enum EventKind {
    NoteOn {
        note: u8,
        velocity: u8,
    },
    NoteOff {
        note: u8,
        velocity: u8,
    },
    ProgramChange {
        program: u8,
    },
    Controller {
        controller: u8,
        value: u8,
    },
    /// Offset from centre, in the raw 14-bit units the wire format uses.
    PitchBend {
        offset: i16,
    },
    ChannelPressure {
        pressure: u8,
    },
    PolyPressure {
        note: u8,
        pressure: u8,
    },
    /// Universal system exclusive; applies to the whole instrument, not a channel.
    MasterVolume {
        level: u16,
    },
    GeneralMidiReset,
}

#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub time_s: f64,
    pub channel: u8,
    pub kind: EventKind,
}

const DEFAULT_TEMPO_US_PER_BEAT: u32 = 500_000;

/// How a file's ticks map onto seconds.
enum Clock {
    /// Ticks divide a beat, so a tempo change rescales every one that follows.
    Metrical {
        ticks_per_beat: f64,
        us_per_beat: f64,
    },
    /// SMPTE ticks are absolute time; tempo messages have no bearing on them.
    Timecode { seconds_per_tick: f64 },
}

impl Clock {
    fn seconds_per_tick(&self) -> f64 {
        match *self {
            Clock::Metrical {
                ticks_per_beat,
                us_per_beat,
            } => (us_per_beat / 1_000_000.0) / ticks_per_beat,
            Clock::Timecode { seconds_per_tick } => seconds_per_tick,
        }
    }

    fn set_tempo(&mut self, microseconds_per_beat: f64) {
        if let Clock::Metrical { us_per_beat, .. } = self {
            *us_per_beat = microseconds_per_beat;
        }
    }
}

fn frames_per_second(fps: Fps) -> f64 {
    match fps {
        // "29" is really 30/1.001, the NTSC drop-frame rate.
        Fps::Fps29 => 30.0 / 1.001,
        other => f64::from(other.as_int()),
    }
}

pub fn load(path: impl AsRef<Path>) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    parse(&bytes)
}

pub fn parse(bytes: &[u8]) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
    let smf = Smf::parse(bytes)?;

    let mut clock = match smf.header.timing {
        Timing::Metrical(t) if t.as_int() > 0 => Clock::Metrical {
            ticks_per_beat: f64::from(t.as_int()),
            us_per_beat: f64::from(DEFAULT_TEMPO_US_PER_BEAT),
        },
        Timing::Metrical(_) => return Err("file declares zero ticks per beat".into()),
        Timing::Timecode(fps, subframe) if subframe > 0 => Clock::Timecode {
            seconds_per_tick: 1.0 / (frames_per_second(fps) * f64::from(subframe)),
        },
        Timing::Timecode(_, _) => return Err("file declares zero subframes per frame".into()),
    };

    // Format 2 tracks are independent sequences rather than parts of one piece,
    // so merging them onto a shared timeline would play them stacked.
    if smf.header.format == Format::Sequential {
        return Err("format 2 (sequentially independent tracks) not supported".into());
    }

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

    for RawEvent { tick, kind } in raw {
        let delta_ticks = (tick - last_tick) as f64;
        current_seconds += delta_ticks * clock.seconds_per_tick();
        last_tick = tick;

        if let TrackEventKind::Meta(MetaMessage::Tempo(t)) = kind {
            clock.set_tempo(f64::from(t.as_int()));
            continue;
        }

        if let TrackEventKind::SysEx(data) = kind {
            let kind = match crate::midi::parse_system_exclusive(data) {
                Some(crate::midi::SystemExclusive::MasterVolume(level)) => {
                    EventKind::MasterVolume { level }
                }
                Some(crate::midi::SystemExclusive::GeneralMidiReset) => EventKind::GeneralMidiReset,
                None => continue,
            };
            events.push(Event {
                time_s: current_seconds,
                channel: 0,
                kind,
            });
            continue;
        }

        let TrackEventKind::Midi { channel, message } = kind else {
            continue;
        };
        let channel = channel.as_int();
        let kind = match message {
            MidiMessage::NoteOn { key, vel } if vel.as_int() == 0 => EventKind::NoteOff {
                note: key.as_int(),
                velocity: 0,
            },
            MidiMessage::NoteOn { key, vel } => EventKind::NoteOn {
                note: key.as_int(),
                velocity: vel.as_int(),
            },
            MidiMessage::NoteOff { key, vel } => EventKind::NoteOff {
                note: key.as_int(),
                velocity: vel.as_int(),
            },
            MidiMessage::ProgramChange { program } => EventKind::ProgramChange {
                program: program.as_int(),
            },
            MidiMessage::Controller { controller, value } => EventKind::Controller {
                controller: controller.as_int(),
                value: value.as_int(),
            },
            // midly already reports the bend centred on zero.
            MidiMessage::PitchBend { bend } => EventKind::PitchBend {
                offset: bend.as_int(),
            },
            MidiMessage::ChannelAftertouch { vel } => EventKind::ChannelPressure {
                pressure: vel.as_int(),
            },
            MidiMessage::Aftertouch { key, vel } => EventKind::PolyPressure {
                note: key.as_int(),
                pressure: vel.as_int(),
            },
        };
        events.push(Event {
            time_s: current_seconds,
            channel,
            kind,
        });
    }

    Ok(events)
}
