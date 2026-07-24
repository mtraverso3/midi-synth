use super::super::oscillator::{SQUARE, Waveform};
use super::{
    BELL, BLOWN, BOWED, BRASS, FOREVER, HIT, Instrument, LEAD, ORGAN, PAD, PLUCKED, REED, STRUCK,
};

/// The 128 General MIDI programs. Each is an archetype plus the handful of
/// overrides that distinguish it from its neighbours.
pub fn voice(program: u8) -> Instrument {
    match program {
        // Piano
        0 => STRUCK,
        1 => Instrument {
            brightness: 18.0,
            ..STRUCK
        }, // bright acoustic
        2 => Instrument {
            decay_s: 1.8,
            brightness: 16.0,
            ..STRUCK
        }, // electric grand
        3 => Instrument {
            decay_s: 1.0,
            brightness: 16.0,
            vibrato_depth: 0.002,
            ..STRUCK
        }, // honky-tonk
        4 => Instrument {
            waveform: Waveform::Sine,
            decay_s: 1.6,
            brightness: 9.0,
            transient: 0.2,
            ..STRUCK
        }, // electric piano 1
        5 => Instrument {
            waveform: Waveform::Sine,
            decay_s: 2.2,
            brightness: 6.0,
            transient: 0.15,
            ..STRUCK
        }, // electric piano 2
        6 => Instrument {
            waveform: Waveform::Pulse(0.28),
            decay_s: 0.7,
            brightness: 10.0,
            brightness_decay_s: 0.15,
            transient: 0.4,
            ..STRUCK
        }, // harpsichord
        7 => Instrument {
            waveform: Waveform::Pulse(0.18),
            decay_s: 0.6,
            brightness: 9.0,
            transient: 0.5,
            ..STRUCK
        }, // clavi

        // Chromatic percussion
        8 => Instrument {
            decay_s: 1.2,
            brightness: 16.0,
            ..BELL
        }, // celesta
        9 => Instrument {
            decay_s: 1.0,
            cutoff_ratio: 6.0,
            brightness: 22.0,
            ..BELL
        }, // glockenspiel
        10 => Instrument {
            decay_s: 1.6,
            brightness: 12.0,
            vibrato_depth: 0.003,
            ..BELL
        }, // music box
        11 => Instrument {
            decay_s: 2.0,
            brightness: 8.0,
            vibrato_depth: 0.008,
            ..BELL
        }, // vibraphone
        12 => Instrument {
            waveform: Waveform::Triangle,
            decay_s: 0.5,
            brightness: 10.0,
            transient: 0.4,
            ..BELL
        }, // marimba
        13 => Instrument {
            waveform: Waveform::Triangle,
            decay_s: 0.3,
            cutoff_ratio: 5.0,
            brightness: 14.0,
            transient: 0.5,
            ..BELL
        }, // xylophone
        14 => Instrument {
            decay_s: 4.0,
            release_s: 2.0,
            brightness: 20.0,
            ..BELL
        }, // tubular bells
        15 => Instrument {
            decay_s: 1.1,
            brightness: 14.0,
            ..PLUCKED
        }, // dulcimer

        // Organ
        16 => ORGAN,
        17 => Instrument {
            transient: 0.35,
            transient_s: 0.05,
            brightness: 4.0,
            ..ORGAN
        }, // percussive organ
        18 => Instrument {
            waveform: Waveform::Pulse(0.35),
            brightness: 5.0,
            ..ORGAN
        }, // rock organ
        19 => Instrument {
            waveform: Waveform::Saw,
            attack_s: 0.09,
            release_s: 0.4,
            brightness: 2.0,
            ..ORGAN
        }, // church organ
        20 => Instrument {
            waveform: Waveform::Saw,
            attack_s: 0.05,
            brightness: 2.2,
            noise_mix: 0.04,
            ..ORGAN
        }, // reed organ
        21 => Instrument {
            waveform: Waveform::Saw,
            attack_s: 0.04,
            brightness: 3.5,
            vibrato_depth: 0.004,
            ..ORGAN
        }, // accordion
        22 => Instrument {
            waveform: Waveform::Pulse(0.22),
            attack_s: 0.03,
            brightness: 4.0,
            vibrato_depth: 0.006,
            noise_mix: 0.05,
            ..ORGAN
        }, // harmonica
        23 => Instrument {
            waveform: Waveform::Saw,
            attack_s: 0.04,
            brightness: 3.0,
            vibrato_depth: 0.005,
            ..ORGAN
        }, // tango accordion

        // Guitar
        24 => Instrument {
            decay_s: 1.1,
            brightness: 9.0,
            ..PLUCKED
        }, // nylon
        25 => Instrument {
            waveform: Waveform::Saw,
            decay_s: 1.3,
            brightness: 13.0,
            ..PLUCKED
        }, // steel
        26 => Instrument {
            waveform: Waveform::Saw,
            decay_s: 1.0,
            brightness: 7.0,
            ..PLUCKED
        }, // jazz
        27 => Instrument {
            waveform: Waveform::Saw,
            decay_s: 1.1,
            brightness: 10.0,
            ..PLUCKED
        }, // clean
        28 => Instrument {
            waveform: Waveform::Pulse(0.3),
            decay_s: 0.22,
            brightness: 6.0,
            transient: 0.55,
            ..PLUCKED
        }, // muted
        29 => Instrument {
            waveform: Waveform::Saw,
            decay_s: 1.6,
            sustain_level: 0.45,
            cutoff_ratio: 3.0,
            brightness: 8.0,
            brightness_decay_s: 0.8,
            body_decay_s: 3.0,
            ..PLUCKED
        }, // overdriven
        30 => Instrument {
            waveform: Waveform::Saw,
            decay_s: 2.0,
            sustain_level: 0.6,
            cutoff_ratio: 3.5,
            brightness: 9.0,
            brightness_decay_s: 1.2,
            body_decay_s: 4.0,
            ..PLUCKED
        }, // distortion
        31 => Instrument {
            waveform: Waveform::Sine,
            decay_s: 1.2,
            cutoff_ratio: 6.0,
            brightness: 6.0,
            ..PLUCKED
        }, // harmonics

        // Bass
        32 => Instrument {
            decay_s: 1.0,
            cutoff_ratio: 2.0,
            brightness: 5.0,
            ..PLUCKED
        }, // acoustic bass
        33 => Instrument {
            decay_s: 1.2,
            cutoff_ratio: 1.8,
            brightness: 6.0,
            ..PLUCKED
        }, // finger bass
        34 => Instrument {
            waveform: Waveform::Saw,
            decay_s: 1.1,
            cutoff_ratio: 1.8,
            brightness: 8.0,
            transient: 0.55,
            ..PLUCKED
        }, // pick bass
        35 => Instrument {
            waveform: Waveform::Sine,
            decay_s: 1.4,
            cutoff_ratio: 2.5,
            brightness: 4.0,
            vibrato_depth: 0.003,
            ..PLUCKED
        }, // fretless
        36 => Instrument {
            waveform: Waveform::Saw,
            decay_s: 0.7,
            cutoff_ratio: 2.0,
            brightness: 14.0,
            brightness_decay_s: 0.1,
            transient: 0.6,
            ..PLUCKED
        }, // slap 1
        37 => Instrument {
            waveform: Waveform::Saw,
            decay_s: 0.5,
            cutoff_ratio: 2.2,
            brightness: 16.0,
            brightness_decay_s: 0.08,
            transient: 0.6,
            ..PLUCKED
        }, // slap 2
        38 => Instrument {
            waveform: SQUARE,
            decay_s: 1.0,
            sustain_level: 0.5,
            cutoff_ratio: 1.6,
            brightness: 5.0,
            ..PLUCKED
        }, // synth bass 1
        39 => Instrument {
            waveform: Waveform::Saw,
            decay_s: 1.2,
            sustain_level: 0.6,
            cutoff_ratio: 1.5,
            brightness: 4.0,
            ..PLUCKED
        }, // synth bass 2

        // Strings
        40 => BOWED,
        41 => Instrument {
            cutoff_ratio: 2.2,
            ..BOWED
        }, // viola
        42 => Instrument {
            attack_s: 0.14,
            cutoff_ratio: 2.0,
            brightness: 3.5,
            ..BOWED
        }, // cello
        43 => Instrument {
            attack_s: 0.16,
            cutoff_ratio: 1.7,
            brightness: 3.0,
            ..BOWED
        }, // contrabass
        44 => Instrument {
            attack_s: 0.05,
            vibrato_depth: 0.012,
            noise_mix: 0.07,
            ..BOWED
        }, // tremolo strings
        45 => Instrument {
            decay_s: 0.5,
            cutoff_ratio: 2.5,
            brightness: 10.0,
            ..PLUCKED
        }, // pizzicato
        46 => Instrument {
            decay_s: 1.8,
            brightness: 11.0,
            transient: 0.3,
            ..PLUCKED
        }, // harp
        47 => Instrument {
            waveform: Waveform::Sine,
            decay_s: 0.9,
            cutoff_ratio: 3.0,
            brightness: 6.0,
            transient: 0.5,
            transient_s: 0.03,
            pitch_drop: 1.2,
            pitch_drop_s: 0.05,
            ..STRUCK
        }, // timpani

        // Ensemble
        48 => Instrument {
            attack_s: 0.2,
            brightness: 3.5,
            ..BOWED
        }, // string ensemble 1
        49 => Instrument {
            attack_s: 0.3,
            brightness: 3.0,
            ..BOWED
        }, // string ensemble 2
        50 => Instrument {
            attack_s: 0.25,
            brightness: 3.0,
            body_decay_s: FOREVER,
            ..BOWED
        }, // synth strings 1
        51 => Instrument {
            attack_s: 0.35,
            brightness: 2.5,
            body_decay_s: FOREVER,
            ..BOWED
        }, // synth strings 2
        52 => Instrument {
            waveform: Waveform::Pulse(0.4),
            attack_s: 0.15,
            brightness: 2.0,
            noise_mix: 0.09,
            vibrato_depth: 0.004,
            ..PAD
        }, // choir aahs
        53 => Instrument {
            waveform: Waveform::Sine,
            attack_s: 0.1,
            brightness: 1.8,
            noise_mix: 0.07,
            ..PAD
        }, // voice oohs
        54 => Instrument {
            waveform: Waveform::Pulse(0.35),
            attack_s: 0.08,
            brightness: 3.0,
            noise_mix: 0.05,
            ..PAD
        }, // synth voice
        55 => Instrument {
            waveform: Waveform::Saw,
            attack_s: 0.005,
            decay_s: 0.3,
            sustain_level: 0.0,
            brightness: 10.0,
            brightness_decay_s: 0.15,
            transient: 0.5,
            ..BRASS
        }, // orchestra hit

        // Brass
        56 => BRASS,
        57 => Instrument {
            cutoff_ratio: 1.5,
            brightness: 6.0,
            ..BRASS
        }, // trombone
        58 => Instrument {
            attack_s: 0.07,
            cutoff_ratio: 1.2,
            brightness: 4.0,
            ..BRASS
        }, // tuba
        59 => Instrument {
            waveform: Waveform::Pulse(0.25),
            cutoff_ratio: 2.5,
            brightness: 5.0,
            transient: 0.3,
            ..BRASS
        }, // muted trumpet
        60 => Instrument {
            attack_s: 0.09,
            cutoff_ratio: 1.5,
            brightness: 4.0,
            ..BRASS
        }, // french horn
        61 => Instrument {
            attack_s: 0.06,
            brightness: 8.0,
            ..BRASS
        }, // brass section
        62 => Instrument {
            attack_s: 0.03,
            brightness: 9.0,
            noise_mix: 0.0,
            pitch_drop: 0.0,
            ..BRASS
        }, // synth brass 1
        63 => Instrument {
            attack_s: 0.04,
            brightness: 7.0,
            noise_mix: 0.0,
            pitch_drop: 0.0,
            ..BRASS
        }, // synth brass 2

        // Reed
        64 => Instrument {
            cutoff_ratio: 2.2,
            ..REED
        }, // soprano sax
        65 => REED, // alto sax
        66 => Instrument {
            cutoff_ratio: 1.8,
            ..REED
        }, // tenor sax
        67 => Instrument {
            cutoff_ratio: 1.6,
            brightness: 2.2,
            ..REED
        }, // baritone sax
        68 => Instrument {
            waveform: Waveform::Pulse(0.2),
            attack_s: 0.04,
            cutoff_ratio: 1.8,
            brightness: 3.0,
            ..REED
        }, // oboe
        69 => Instrument {
            waveform: Waveform::Pulse(0.25),
            attack_s: 0.04,
            cutoff_ratio: 1.7,
            brightness: 3.0,
            ..REED
        }, // english horn
        70 => Instrument {
            waveform: Waveform::Pulse(0.3),
            attack_s: 0.05,
            cutoff_ratio: 1.4,
            brightness: 3.5,
            ..REED
        }, // bassoon
        71 => Instrument {
            waveform: SQUARE,
            attack_s: 0.05,
            cutoff_ratio: 1.5,
            brightness: 1.5,
            brightness_decay_s: 0.4,
            transient: 0.2,
            transient_s: 0.07,
            vibrato_depth: 0.002,
            ..REED
        }, // clarinet: hollow, odd harmonics only

        // Pipe
        72 => Instrument {
            attack_s: 0.03,
            cutoff_ratio: 3.5,
            noise_mix: 0.13,
            ..BLOWN
        }, // piccolo
        73 => BLOWN, // flute
        74 => Instrument {
            noise_mix: 0.14,
            brightness: 2.5,
            ..BLOWN
        }, // recorder
        75 => Instrument {
            noise_mix: 0.2,
            transient: 0.4,
            ..BLOWN
        }, // pan flute
        76 => Instrument {
            noise_mix: 0.3,
            attack_s: 0.09,
            transient: 0.45,
            ..BLOWN
        }, // blown bottle
        77 => Instrument {
            noise_mix: 0.25,
            attack_s: 0.08,
            vibrato_depth: 0.007,
            ..BLOWN
        }, // shakuhachi
        78 => Instrument {
            noise_mix: 0.08,
            cutoff_ratio: 4.0,
            vibrato_depth: 0.006,
            ..BLOWN
        }, // whistle
        79 => Instrument {
            noise_mix: 0.06,
            brightness: 2.0,
            ..BLOWN
        }, // ocarina

        // Synth lead
        80 => LEAD,
        81 => Instrument {
            waveform: Waveform::Saw,
            ..LEAD
        }, // saw lead
        82 => Instrument {
            waveform: Waveform::Triangle,
            noise_mix: 0.06,
            ..LEAD
        }, // calliope
        83 => Instrument {
            waveform: Waveform::Pulse(0.3),
            transient: 0.4,
            transient_s: 0.05,
            ..LEAD
        }, // chiff
        84 => Instrument {
            waveform: Waveform::Saw,
            brightness: 9.0,
            ..LEAD
        }, // charang
        85 => Instrument {
            waveform: Waveform::Pulse(0.35),
            noise_mix: 0.07,
            attack_s: 0.06,
            ..LEAD
        }, // voice lead
        86 => Instrument {
            waveform: Waveform::Saw,
            brightness: 5.0,
            ..LEAD
        }, // fifths
        87 => Instrument {
            waveform: Waveform::Saw,
            cutoff_ratio: 1.6,
            brightness: 5.0,
            ..LEAD
        }, // bass + lead

        // Synth pad
        88 => PAD,
        89 => Instrument {
            attack_s: 0.7,
            brightness: 2.0,
            ..PAD
        }, // warm
        90 => Instrument {
            waveform: Waveform::Pulse(0.4),
            attack_s: 0.3,
            brightness: 4.0,
            ..PAD
        }, // polysynth
        91 => Instrument {
            attack_s: 0.6,
            noise_mix: 0.06,
            brightness: 2.0,
            ..PAD
        }, // choir pad
        92 => Instrument {
            attack_s: 0.45,
            noise_mix: 0.05,
            vibrato_depth: 0.005,
            ..PAD
        }, // bowed pad
        93 => Instrument {
            waveform: SQUARE,
            attack_s: 0.35,
            brightness: 6.0,
            ..PAD
        }, // metallic
        94 => Instrument {
            attack_s: 0.8,
            brightness: 2.5,
            vibrato_depth: 0.004,
            ..PAD
        }, // halo
        95 => Instrument {
            attack_s: 0.55,
            brightness_decay_s: 4.0,
            brightness: 8.0,
            ..PAD
        }, // sweep

        // Synth effects
        96 => Instrument {
            noise_mix: 0.3,
            attack_s: 0.4,
            brightness: 6.0,
            ..PAD
        }, // rain
        97 => Instrument {
            attack_s: 0.6,
            brightness: 5.0,
            ..PAD
        }, // soundtrack
        98 => Instrument {
            decay_s: 2.0,
            brightness: 20.0,
            ..BELL
        }, // crystal
        99 => Instrument {
            attack_s: 0.5,
            noise_mix: 0.08,
            brightness: 4.0,
            ..PAD
        }, // atmosphere
        100 => Instrument {
            attack_s: 0.2,
            brightness: 12.0,
            brightness_decay_s: 3.0,
            ..PAD
        }, // brightness
        101 => Instrument {
            attack_s: 0.5,
            vibrato_depth: 0.02,
            brightness: 3.0,
            ..PAD
        }, // goblins
        102 => Instrument {
            decay_s: 2.2,
            brightness: 10.0,
            ..BELL
        }, // echoes
        103 => Instrument {
            waveform: Waveform::Saw,
            attack_s: 0.3,
            brightness: 7.0,
            vibrato_depth: 0.01,
            ..PAD
        }, // sci-fi

        // Ethnic
        104 => Instrument {
            waveform: Waveform::Saw,
            decay_s: 1.6,
            brightness: 13.0,
            transient: 0.5,
            vibrato_depth: 0.004,
            ..PLUCKED
        }, // sitar
        105 => Instrument {
            waveform: Waveform::Pulse(0.25),
            decay_s: 0.6,
            brightness: 14.0,
            transient: 0.55,
            ..PLUCKED
        }, // banjo
        106 => Instrument {
            waveform: Waveform::Pulse(0.3),
            decay_s: 0.7,
            brightness: 10.0,
            transient: 0.5,
            ..PLUCKED
        }, // shamisen
        107 => Instrument {
            decay_s: 1.0,
            brightness: 11.0,
            transient: 0.45,
            ..PLUCKED
        }, // koto
        108 => Instrument {
            waveform: Waveform::Sine,
            decay_s: 0.6,
            brightness: 8.0,
            transient: 0.3,
            ..PLUCKED
        }, // kalimba
        109 => Instrument {
            waveform: Waveform::Pulse(0.22),
            attack_s: 0.03,
            brightness: 4.0,
            noise_mix: 0.08,
            vibrato_depth: 0.002,
            ..REED
        }, // bagpipe
        110 => Instrument {
            attack_s: 0.06,
            brightness: 6.0,
            vibrato_depth: 0.008,
            ..BOWED
        }, // fiddle
        111 => Instrument {
            waveform: Waveform::Pulse(0.18),
            cutoff_ratio: 2.2,
            brightness: 4.0,
            ..REED
        }, // shanai

        // Percussive
        112 => Instrument {
            decay_s: 0.8,
            cutoff_ratio: 6.0,
            brightness: 20.0,
            ..BELL
        }, // tinkle bell
        113 => Instrument {
            waveform: Waveform::Pulse(0.25),
            decay_s: 0.35,
            brightness: 12.0,
            transient: 0.5,
            ..BELL
        }, // agogo
        114 => Instrument {
            decay_s: 1.2,
            cutoff_ratio: 3.0,
            brightness: 12.0,
            transient: 0.45,
            ..BELL
        }, // steel drums
        115 => Instrument {
            waveform: Waveform::Triangle,
            decay_s: 0.12,
            brightness: 10.0,
            transient: 0.6,
            ..BELL
        }, // woodblock
        116 => Instrument {
            waveform: Waveform::Sine,
            decay_s: 0.6,
            cutoff_ratio: 2.0,
            brightness: 4.0,
            transient: 0.5,
            transient_s: 0.02,
            pitch_drop: 2.0,
            pitch_drop_s: 0.04,
            ..STRUCK
        }, // taiko
        117 => Instrument {
            waveform: Waveform::Sine,
            decay_s: 0.4,
            cutoff_ratio: 2.5,
            brightness: 5.0,
            transient: 0.4,
            pitch_drop: 3.0,
            pitch_drop_s: 0.05,
            ..STRUCK
        }, // melodic tom
        118 => Instrument {
            waveform: Waveform::Sine,
            decay_s: 0.35,
            brightness: 6.0,
            transient: 0.3,
            pitch_drop: 6.0,
            pitch_drop_s: 0.06,
            ..STRUCK
        }, // synth drum
        119 => Instrument {
            waveform: Waveform::Noise,
            attack_s: 0.35,
            decay_s: 0.05,
            sustain_level: 0.0,
            release_s: 0.05,
            cutoff_ratio: 30.0,
            brightness: 6.0,
            brightness_decay_s: 2.0,
            fixed_pitch: Some(300.0),
            ..HIT
        }, // reverse cymbal

        // Sound effects
        120 => Instrument {
            waveform: Waveform::Noise,
            decay_s: 0.2,
            cutoff_ratio: 12.0,
            brightness: 8.0,
            fixed_pitch: Some(300.0),
            ..HIT
        }, // fret noise
        121 => Instrument {
            waveform: Waveform::Noise,
            attack_s: 0.05,
            decay_s: 0.5,
            cutoff_ratio: 8.0,
            brightness: 4.0,
            fixed_pitch: Some(400.0),
            ..HIT
        }, // breath noise
        122 => Instrument {
            waveform: Waveform::Noise,
            attack_s: 0.9,
            decay_s: 1.5,
            sustain_level: 0.4,
            release_s: 1.2,
            cutoff_ratio: 6.0,
            brightness: 4.0,
            fixed_pitch: Some(500.0),
            ..HIT
        }, // seashore
        123 => Instrument {
            decay_s: 0.15,
            cutoff_ratio: 8.0,
            brightness: 6.0,
            vibrato_depth: 0.05,
            ..BELL
        }, // bird tweet
        124 => Instrument {
            waveform: SQUARE,
            decay_s: 0.3,
            sustain_level: 0.7,
            cutoff_ratio: 2.0,
            brightness: 1.0,
            fixed_pitch: Some(1000.0),
            ..LEAD
        }, // telephone ring
        125 => Instrument {
            waveform: Waveform::Noise,
            attack_s: 0.2,
            sustain_level: 0.8,
            release_s: 0.4,
            cutoff_ratio: 1.5,
            brightness: 3.0,
            fixed_pitch: Some(120.0),
            ..HIT
        }, // helicopter
        126 => Instrument {
            waveform: Waveform::Noise,
            attack_s: 0.4,
            decay_s: 1.0,
            sustain_level: 0.6,
            release_s: 0.8,
            cutoff_ratio: 10.0,
            brightness: 5.0,
            fixed_pitch: Some(400.0),
            ..HIT
        }, // applause
        _ => Instrument {
            waveform: Waveform::Noise,
            decay_s: 0.25,
            cutoff_ratio: 4.0,
            brightness: 14.0,
            brightness_decay_s: 0.05,
            fixed_pitch: Some(200.0),
            ..HIT
        }, // gunshot
    }
}
