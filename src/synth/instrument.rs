use super::oscillator::Waveform;

/// General MIDI reserves channel 9 for percussion, regardless of program.
const DRUM_CHANNEL: u8 = 9;

#[derive(Clone, Copy)]
pub struct Instrument {
    pub waveform: Waveform,
    pub attack_s: f32,
    pub decay_s: f32,
    pub sustain_level: f32,
    pub release_s: f32,
    pub cutoff_hz: f32,
}

/// Percussive: strikes fast and decays to silence while held (no sustain).
const PLUCKED: fn(Waveform) -> Instrument = |waveform| Instrument {
    waveform,
    attack_s: 0.005,
    decay_s: 0.5,
    sustain_level: 0.0,
    release_s: 0.2,
    cutoff_hz: 6000.0,
};

/// Sustained: eases in and holds at full level until released.
const SUSTAINED: fn(Waveform) -> Instrument = |waveform| Instrument {
    waveform,
    attack_s: 0.06,
    decay_s: 0.1,
    sustain_level: 0.8,
    release_s: 0.3,
    cutoff_hz: 3500.0,
};

/// Short General MIDI family label, for display.
pub fn family_name(channel: u8, program: u8) -> &'static str {
    if channel == DRUM_CHANNEL {
        return "Drums";
    }
    match program {
        0..=7 => "Piano",
        8..=15 => "Chroma",
        16..=23 => "Organ",
        24..=31 => "Guitar",
        32..=39 => "Bass",
        40..=47 => "Strings",
        48..=55 => "Ensemble",
        56..=63 => "Brass",
        64..=71 => "Reed",
        72..=79 => "Pipe",
        80..=87 => "Lead",
        88..=95 => "Pad",
        _ => "Synth",
    }
}

/// Map a General MIDI program to a synth voice, by instrument family.
pub fn for_channel(channel: u8, program: u8) -> Instrument {
    if channel == DRUM_CHANNEL {
        return Instrument {
            waveform: Waveform::Noise,
            attack_s: 0.001,
            decay_s: 0.12,
            sustain_level: 0.0,
            release_s: 0.05,
            cutoff_hz: 8000.0,
        };
    }
    match program {
        0..=15 => PLUCKED(Waveform::Triangle), // pianos, chromatic percussion
        16..=23 => SUSTAINED(Waveform::Sine),  // organs
        24..=39 => PLUCKED(Waveform::Triangle), // guitars, basses
        40..=63 => SUSTAINED(Waveform::Saw),   // strings, ensembles, brass
        64..=71 => SUSTAINED(Waveform::Square), // reeds
        72..=79 => SUSTAINED(Waveform::Sine),  // pipes, flutes
        _ => SUSTAINED(Waveform::Square),      // synth leads and everything else
    }
}
