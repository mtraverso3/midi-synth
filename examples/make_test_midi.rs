//! Generates `assets/twinkle.mid`. Run with: cargo run --example make_test_midi

use std::fs;

use midly::num::{u4, u7, u15, u24, u28};
use midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, Track, TrackEvent, TrackEventKind,
};

const TICKS_PER_BEAT: u16 = 480;

fn main() {
    // (MIDI note, beats)
    let melody: &[(u8, u32)] = &[
        (60, 1),
        (60, 1),
        (67, 1),
        (67, 1),
        (69, 1),
        (69, 1),
        (67, 2),
        (65, 1),
        (65, 1),
        (64, 1),
        (64, 1),
        (62, 1),
        (62, 1),
        (60, 2),
    ];

    let mut track = Track::new();

    track.push(TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::new(500_000))),
    });

    for &(note, beats) in melody {
        let duration = beats * TICKS_PER_BEAT as u32;
        track.push(TrackEvent {
            delta: u28::new(0),
            kind: TrackEventKind::Midi {
                channel: u4::new(0),
                message: MidiMessage::NoteOn {
                    key: u7::new(note),
                    vel: u7::new(96),
                },
            },
        });
        track.push(TrackEvent {
            delta: u28::new(duration),
            kind: TrackEventKind::Midi {
                channel: u4::new(0),
                message: MidiMessage::NoteOff {
                    key: u7::new(note),
                    vel: u7::new(0),
                },
            },
        });
    }

    track.push(TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });

    let smf = Smf {
        header: Header::new(
            Format::SingleTrack,
            Timing::Metrical(u15::new(TICKS_PER_BEAT)),
        ),
        tracks: vec![track],
    };

    fs::create_dir_all("assets").unwrap();
    smf.save("assets/twinkle.mid").unwrap();
    println!("Wrote assets/twinkle.mid");
}
