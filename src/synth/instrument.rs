use super::oscillator::Waveform;

/// MIDI channel 9 (0-indexed) is reserved for percussion by the General MIDI
/// standard, regardless of program.
const DRUM_CHANNEL: u8 = 9;

/// A sound design: which wave to use and how its amplitude evolves over a note.
#[derive(Clone, Copy)]
pub struct Instrument {
    pub waveform: Waveform,
    pub attack_s: f32,
    pub decay_s: f32,
    pub sustain_level: f32,
    pub release_s: f32,
}

/// Percussive: strikes fast and decays to silence while held (no sustain).
const PLUCKED: fn(Waveform) -> Instrument = |waveform| Instrument {
    waveform,
    attack_s: 0.005,
    decay_s: 0.5,
    sustain_level: 0.0,
    release_s: 0.2,
};

/// Sustained: eases in and holds at full level until released.
const SUSTAINED: fn(Waveform) -> Instrument = |waveform| Instrument {
    waveform,
    attack_s: 0.06,
    decay_s: 0.1,
    sustain_level: 0.8,
    release_s: 0.3,
};

/// Pick an instrument for a channel from its General MIDI program number,
/// grouping programs into families that share a timbre and amplitude shape.
pub fn for_channel(channel: u8, program: u8) -> Instrument {
    if channel == DRUM_CHANNEL {
        return Instrument {
            waveform: Waveform::Noise,
            attack_s: 0.001,
            decay_s: 0.12,
            sustain_level: 0.0,
            release_s: 0.05,
        };
    }
    match program {
        0..=15 => PLUCKED(Waveform::Triangle),  // pianos, chromatic percussion
        16..=23 => SUSTAINED(Waveform::Sine),   // organs
        24..=39 => PLUCKED(Waveform::Triangle), // guitars, basses
        40..=63 => SUSTAINED(Waveform::Saw),    // strings, ensembles, brass
        64..=71 => SUSTAINED(Waveform::Square), // reeds
        72..=79 => SUSTAINED(Waveform::Sine),   // pipes, flutes
        _ => SUSTAINED(Waveform::Square),       // synth leads and everything else
    }
}
