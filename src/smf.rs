use std::path::Path;

use midly::{Format, Fps, MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};

use crate::midi::{CENTRE_14_BIT, SystemExclusive};

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
    /// Left/right balance, -1.0 hard left to 1.0 hard right.
    MasterBalance {
        position: f32,
    },
    /// Global detune in semitones.
    MasterTuning {
        semitones: f32,
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
    enum Raw<'a> {
        Track(TrackEventKind<'a>),
        SysEx(Vec<u8>),
    }
    struct RawEvent<'a> {
        tick: u64,
        kind: Raw<'a>,
    }
    let mut raw = Vec::new();
    for track in &smf.tracks {
        let mut abs_tick = 0u64;
        let mut pending: Option<Vec<u8>> = None;
        for event in track {
            abs_tick += event.delta.as_int() as u64;

            // A SysEx packet not ending in 0xF7 is continued by the escape
            // events after it, which carry no status byte of their own.
            let packet = match event.kind {
                TrackEventKind::SysEx(data) => Some((Vec::new(), data)),
                TrackEventKind::Escape(data) => pending.take().map(|held| (held, data)),
                _ => None,
            };
            if let Some((mut buffer, data)) = packet {
                buffer.extend_from_slice(data);
                match buffer.last() {
                    Some(&0xF7) => raw.push(RawEvent {
                        tick: abs_tick,
                        kind: Raw::SysEx(buffer),
                    }),
                    _ => pending = Some(buffer),
                }
                continue;
            }

            if let TrackEventKind::Meta(MetaMessage::EndOfTrack) = event.kind {
                break;
            }
            raw.push(RawEvent {
                tick: abs_tick,
                kind: Raw::Track(event.kind),
            });
        }
    }
    // Stable, so each track's own order survives where ticks coincide.
    raw.sort_by_key(|e| e.tick);

    // Walk the timeline converting ticks to seconds, tracking tempo changes.
    let mut events = Vec::new();
    let mut current_seconds = 0.0f64;
    let mut last_tick = 0u64;

    let mut coarse_tuning = 0.0f32;
    let mut fine_tuning = 0.0f32;

    for RawEvent { tick, kind } in raw {
        let delta_ticks = (tick - last_tick) as f64;
        current_seconds += delta_ticks * clock.seconds_per_tick();
        last_tick = tick;

        let kind = match kind {
            Raw::SysEx(data) => {
                let Some(message) = crate::midi::parse_system_exclusive(&data) else {
                    continue;
                };
                let kind = match message {
                    SystemExclusive::MasterVolume(level) => EventKind::MasterVolume { level },
                    SystemExclusive::MasterBalance(position) => EventKind::MasterBalance {
                        position: (i32::from(position) - CENTRE_14_BIT) as f32 / 8192.0,
                    },
                    SystemExclusive::MasterFineTuning(raw) => {
                        fine_tuning = (i32::from(raw) - CENTRE_14_BIT) as f32 / 8192.0;
                        EventKind::MasterTuning {
                            semitones: coarse_tuning + fine_tuning,
                        }
                    }
                    SystemExclusive::MasterCoarseTuning(msb) => {
                        coarse_tuning = f32::from(msb) - 64.0;
                        EventKind::MasterTuning {
                            semitones: coarse_tuning + fine_tuning,
                        }
                    }
                    SystemExclusive::GeneralMidiReset => {
                        coarse_tuning = 0.0;
                        fine_tuning = 0.0;
                        EventKind::GeneralMidiReset
                    }
                };
                events.push(Event {
                    time_s: current_seconds,
                    channel: 0,
                    kind,
                });
                continue;
            }
            Raw::Track(kind) => kind,
        };

        if let TrackEventKind::Meta(MetaMessage::Tempo(t)) = kind {
            clock.set_tempo(f64::from(t.as_int()));
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

#[cfg(test)]
mod tests {
    use super::*;
    use midly::num::{u4, u7, u15, u24, u28};
    use midly::{Header, Track, TrackEvent};

    fn at(delta: u32, kind: TrackEventKind<'_>) -> TrackEvent<'_> {
        TrackEvent {
            delta: u28::new(delta),
            kind,
        }
    }

    fn note_on(delta: u32, note: u8, velocity: u8) -> TrackEvent<'static> {
        at(
            delta,
            TrackEventKind::Midi {
                channel: u4::new(0),
                message: MidiMessage::NoteOn {
                    key: u7::new(note),
                    vel: u7::new(velocity),
                },
            },
        )
    }

    fn build(timing: Timing, format: Format, tracks: Vec<Track<'_>>) -> Vec<Event> {
        let mut bytes = Vec::new();
        Smf {
            header: Header::new(format, timing),
            tracks,
        }
        .write(&mut bytes)
        .unwrap();
        parse(&bytes).unwrap()
    }

    fn metrical(track: Track<'_>) -> Vec<Event> {
        build(
            Timing::Metrical(u15::new(480)),
            Format::SingleTrack,
            vec![track],
        )
    }

    #[test]
    fn a_note_on_at_velocity_zero_is_a_note_off() {
        let events = metrical(vec![
            note_on(0, 60, 64),
            note_on(480, 60, 0),
            at(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
        ]);
        assert!(matches!(events[0].kind, EventKind::NoteOn { note: 60, .. }));
        assert!(matches!(events[1].kind, EventKind::NoteOff { note: 60, .. }));
    }

    #[test]
    fn tempo_rescales_only_what_follows_it() {
        let events = metrical(vec![
            note_on(0, 60, 64),
            // One beat at the 120bpm default, then half speed for the next.
            note_on(480, 61, 64),
            at(
                0,
                TrackEventKind::Meta(MetaMessage::Tempo(u24::new(1_000_000))),
            ),
            note_on(480, 62, 64),
            at(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
        ]);
        assert!((events[0].time_s - 0.0).abs() < 1e-9);
        assert!((events[1].time_s - 0.5).abs() < 1e-9);
        assert!((events[2].time_s - 1.5).abs() < 1e-9);
    }

    #[test]
    fn timecode_ticks_ignore_tempo() {
        let events = build(
            Timing::Timecode(Fps::Fps25, 40),
            Format::SingleTrack,
            vec![vec![
                at(
                    0,
                    TrackEventKind::Meta(MetaMessage::Tempo(u24::new(1_000_000))),
                ),
                note_on(1000, 60, 64),
                at(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
            ]],
        );
        assert!((events[0].time_s - 1.0).abs() < 1e-9);
    }

    #[test]
    fn events_after_end_of_track_are_dropped() {
        let events = metrical(vec![
            note_on(0, 60, 64),
            at(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
            note_on(0, 62, 64),
        ]);
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn tracks_merge_onto_one_timeline() {
        let events = build(
            Timing::Metrical(u15::new(480)),
            Format::Parallel,
            vec![
                vec![
                    note_on(480, 60, 64),
                    at(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
                ],
                vec![
                    note_on(0, 72, 64),
                    at(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
                ],
            ],
        );
        let notes: Vec<_> = events
            .iter()
            .map(|e| match e.kind {
                EventKind::NoteOn { note, .. } => note,
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(notes, [72, 60]);
    }

    #[test]
    fn a_sysex_split_across_packets_is_reassembled() {
        let events = metrical(vec![
            at(0, TrackEventKind::SysEx(&[0x7E, 0x7F])),
            at(0, TrackEventKind::Escape(&[0x09, 0x01, 0xF7])),
            at(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
        ]);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].kind, EventKind::GeneralMidiReset));
    }

    #[test]
    fn coarse_and_fine_master_tuning_combine() {
        let events = metrical(vec![
            // +2 semitones coarse, then half a semitone sharp on top.
            at(
                0,
                TrackEventKind::SysEx(&[0x7F, 0x7F, 0x04, 0x04, 0x00, 66, 0xF7]),
            ),
            at(
                0,
                TrackEventKind::SysEx(&[0x7F, 0x7F, 0x04, 0x03, 0x00, 0x60, 0xF7]),
            ),
            at(0, TrackEventKind::Meta(MetaMessage::EndOfTrack)),
        ]);
        let EventKind::MasterTuning { semitones } = events[1].kind else {
            panic!("expected a tuning event");
        };
        assert!((semitones - 2.5).abs() < 1e-6);
    }

    #[test]
    fn format_2_and_degenerate_timing_are_rejected() {
        let mut bytes = Vec::new();
        Smf {
            header: Header::new(Format::Sequential, Timing::Metrical(u15::new(480))),
            tracks: vec![vec![at(0, TrackEventKind::Meta(MetaMessage::EndOfTrack))]],
        }
        .write(&mut bytes)
        .unwrap();
        assert!(parse(&bytes).is_err());
    }
}
