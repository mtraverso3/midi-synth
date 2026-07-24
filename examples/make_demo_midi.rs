//! Generates `assets/demo.mid`: a 4-bar loop with parts on separate MIDI
//! channels, each a different instrument, with per-channel volume and a sustain
//! pedal on the lead. Run with: cargo run --example make_demo_midi

use std::fs;

use midly::num::{u4, u7, u15, u24, u28};
use midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, Track, TrackEvent, TrackEventKind,
};

const TICKS_PER_BEAT: u16 = 480;
const BEAT: u32 = TICKS_PER_BEAT as u32;
const BAR: u32 = 4 * BEAT;

const CC_VOLUME: u8 = 7;
const CC_SUSTAIN: u8 = 64;

/// (start_tick, duration_tick, note)
type Note = (u32, u32, u8);
/// (tick, controller, value)
type Control = (u32, u8, u8);

fn main() {
    // Chord roots per bar: C, Am, F, G.
    let chords: [[u8; 3]; 4] = [[60, 64, 67], [57, 60, 64], [53, 57, 60], [55, 59, 62]];

    let mut pad = Vec::new();
    let mut bass = Vec::new();
    let mut lead = Vec::new();
    let mut drums = Vec::new();
    let mut lead_pedal = Vec::new();

    for (bar, chord) in chords.iter().enumerate() {
        let bar_start = bar as u32 * BAR;

        for beat in 0..4 {
            let t = bar_start + beat * BEAT;
            let drum = if beat % 2 == 0 { 36 } else { 38 };
            drums.push((t, BEAT / 4, drum));
            drums.push((t + BEAT / 2, BEAT / 4, 42));
        }

        for &note in chord {
            pad.push((bar_start, BAR, note));
        }

        for beat in 0..4 {
            bass.push((bar_start + beat * BEAT, BEAT, chord[0] - 24));
        }

        let arp = [chord[0], chord[1], chord[2], chord[1]];
        for (i, &note) in arp.iter().enumerate() {
            lead.push((bar_start + i as u32 * BEAT, BEAT / 2, note + 12));
        }

        // Hold the pedal through the bar so the lead notes ring into each other.
        lead_pedal.push((bar_start, CC_SUSTAIN, 127));
        lead_pedal.push((bar_start + BAR - 1, CC_SUSTAIN, 0));
    }

    let smf = Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::new(TICKS_PER_BEAT))),
        tracks: vec![
            build_track(0, 48, 100, &lead, &lead_pedal), // strings -> saw
            build_track(1, 0, 85, &pad, &[]),            // piano   -> triangle
            build_track(2, 18, 110, &bass, &[]),         // organ   -> sine
            build_track(9, 0, 100, &drums, &[]),         // channel 9 -> noise
        ],
    };

    fs::create_dir_all("assets").unwrap();
    smf.save("assets/demo.mid").unwrap();
    println!("Wrote assets/demo.mid");
}

fn build_track(
    channel: u8,
    program: u8,
    volume: u8,
    notes: &[Note],
    controls: &[Control],
) -> Track<'static> {
    let ch = u4::new(channel);

    // Collect every message with its absolute tick and an ordering key (so at a
    // shared tick, controllers and note-offs come before note-ons).
    let mut msgs: Vec<(u32, u8, MidiMessage)> = vec![
        (
            0,
            0,
            MidiMessage::ProgramChange {
                program: u7::new(program),
            },
        ),
        (0, 0, controller(CC_VOLUME, volume)),
    ];
    for &(tick, cc, value) in controls {
        msgs.push((tick, 0, controller(cc, value)));
    }
    for &(start, dur, note) in notes {
        msgs.push((start + dur, 1, note_off(note)));
        msgs.push((start, 2, note_on(note)));
    }
    msgs.sort_by_key(|&(tick, order, _)| (tick, order));

    let mut track = Track::new();
    track.push(TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::new(500_000))),
    });

    let mut last_tick = 0u32;
    for (tick, _, message) in msgs {
        track.push(TrackEvent {
            delta: u28::new(tick - last_tick),
            kind: TrackEventKind::Midi {
                channel: ch,
                message,
            },
        });
        last_tick = tick;
    }

    track.push(TrackEvent {
        delta: u28::new(0),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });
    track
}

fn note_on(note: u8) -> MidiMessage {
    MidiMessage::NoteOn {
        key: u7::new(note),
        vel: u7::new(90),
    }
}

fn note_off(note: u8) -> MidiMessage {
    MidiMessage::NoteOff {
        key: u7::new(note),
        vel: u7::new(0),
    }
}

fn controller(controller: u8, value: u8) -> MidiMessage {
    MidiMessage::Controller {
        controller: u7::new(controller),
        value: u7::new(value),
    }
}
