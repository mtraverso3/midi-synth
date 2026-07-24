//! Generates `assets/demo.mid`: a 4-bar loop with three parts on three MIDI
//! channels, each a different instrument family. Run with:
//! cargo run --example make_demo_midi

use std::fs;

use midly::num::{u4, u7, u15, u24, u28};
use midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, Track, TrackEvent, TrackEventKind,
};

const TICKS_PER_BEAT: u16 = 480;
const BEAT: u32 = TICKS_PER_BEAT as u32;
const BAR: u32 = 4 * BEAT;

/// (start_tick, duration_tick, note)
type Note = (u32, u32, u8);

fn main() {
    // Chord roots per bar: C, Am, F, G.
    let chords: [[u8; 3]; 4] = [[60, 64, 67], [57, 60, 64], [53, 57, 60], [55, 59, 62]];

    let mut pad = Vec::new();
    let mut bass = Vec::new();
    let mut lead = Vec::new();
    let mut drums = Vec::new();

    for (bar, chord) in chords.iter().enumerate() {
        let bar_start = bar as u32 * BAR;

        // Drums (noise bursts; pitch is ignored): kick on 1 & 3, snare on 2 & 4,
        // hi-hats every half beat.
        for beat in 0..4 {
            let t = bar_start + beat * BEAT;
            let drum = if beat % 2 == 0 { 36 } else { 38 };
            drums.push((t, BEAT / 4, drum));
            drums.push((t + BEAT / 2, BEAT / 4, 42));
        }

        // Pad: whole-note chord held across the bar.
        for &note in chord {
            pad.push((bar_start, BAR, note));
        }

        // Bass: root two octaves down, one hit per beat.
        for beat in 0..4 {
            bass.push((bar_start + beat * BEAT, BEAT, chord[0] - 24));
        }

        // Lead: an arpeggio of the chord tones an octave up, eighth notes.
        let arp = [chord[0], chord[1], chord[2], chord[1]];
        for (i, &note) in arp.iter().enumerate() {
            let start = bar_start + i as u32 * 2 * (BEAT / 2);
            lead.push((start, BEAT / 2, note + 12));
        }
    }

    let smf = Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(TICKS_PER_BEAT))),
        tracks: vec![
            build_track(0, 48, &lead), // strings -> saw
            build_track(1, 0, &pad),   // piano   -> triangle
            build_track(2, 18, &bass), // organ   -> sine
            build_track(9, 0, &drums), // channel 9 -> noise percussion
        ],
    };

    fs::create_dir_all("assets").unwrap();
    smf.save("assets/demo.mid").unwrap();
    println!("Wrote assets/demo.mid");
}

fn build_track(channel: u8, program: u8, notes: &[Note]) -> Track<'static> {
    // Expand notes into absolute-tick on/off points, then sort so we can
    // delta-encode a single ordered stream (parts overlap, e.g. chords).
    let mut points: Vec<(u32, bool, u8)> = Vec::new();
    for &(start, dur, note) in notes {
        points.push((start, true, note));
        points.push((start + dur, false, note));
    }
    points.sort_by_key(|&(tick, on, _)| (tick, on));

    let ch = u4::new(channel);
    let mut track = Track::new();

    track.push(TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::new(500_000))),
    });
    track.push(TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Midi {
            channel: ch,
            message: MidiMessage::ProgramChange {
                program: u7::new(program),
            },
        },
    });

    let mut last_tick = 0u32;
    for (tick, on, note) in points {
        let message = if on {
            MidiMessage::NoteOn {
                key: u7::new(note),
                vel: u7::new(90),
            }
        } else {
            MidiMessage::NoteOff {
                key: u7::new(note),
                vel: u7::new(0),
            }
        };
        track.push(TrackEvent {
            delta: u28::new(tick - last_tick),
            kind: TrackEventKind::Midi { channel: ch, message },
        });
        last_tick = tick;
    }

    track.push(TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });
    track
}
