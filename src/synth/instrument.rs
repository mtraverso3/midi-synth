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
    /// Cutoff once the filter envelope has closed, as a multiple of the note's pitch.
    pub cutoff_ratio: f32,
    /// Extra cutoff at the peak of the filter envelope, as a further multiple.
    pub brightness: f32,
    /// How fast the filter envelope closes, darkening the note as it rings.
    pub brightness_decay_s: f32,
    /// Level of the inharmonic noise burst at the attack.
    pub transient: f32,
    pub transient_s: f32,
    /// Time constant for a held note losing energy; large means it holds forever.
    pub body_decay_s: f32,
    pub vibrato_depth: f32,
}

const BASE: Instrument = Instrument {
    waveform: Waveform::Triangle,
    attack_s: 0.005,
    decay_s: 0.5,
    sustain_level: 0.0,
    release_s: 0.2,
    cutoff_ratio: 3.0,
    brightness: 8.0,
    brightness_decay_s: 0.4,
    transient: 0.0,
    transient_s: 0.01,
    body_decay_s: FOREVER,
    vibrato_depth: 0.0,
};

const FOREVER: f32 = 1.0e9;

/// Struck or plucked: a bright inharmonic onset that decays to silence while held.
const PLUCKED: fn(Waveform) -> Instrument = |waveform| Instrument {
    waveform,
    attack_s: 0.002,
    decay_s: 1.4,
    sustain_level: 0.0,
    release_s: 0.25,
    cutoff_ratio: 2.5,
    brightness: 14.0,
    brightness_decay_s: 0.25,
    transient: 0.35,
    transient_s: 0.012,
    ..BASE
};

/// Blown or bowed: eases in, holds while pressed, and drifts under vibrato.
const SUSTAINED: fn(Waveform) -> Instrument = |waveform| Instrument {
    waveform,
    attack_s: 0.07,
    decay_s: 0.25,
    sustain_level: 0.75,
    release_s: 0.35,
    cutoff_ratio: 3.5,
    brightness: 6.0,
    brightness_decay_s: 0.9,
    transient: 0.12,
    transient_s: 0.05,
    body_decay_s: 6.0,
    vibrato_depth: 0.004,
};

const DRUM: Instrument = Instrument {
    waveform: Waveform::Noise,
    attack_s: 0.001,
    decay_s: 0.12,
    sustain_level: 0.0,
    release_s: 0.05,
    cutoff_ratio: 60.0,
    brightness: 5.0,
    brightness_decay_s: 0.06,
    transient: 0.0,
    transient_s: 0.01,
    body_decay_s: FOREVER,
    vibrato_depth: 0.0,
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
        return DRUM;
    }
    match program {
        0..=7 => PLUCKED(Waveform::Triangle), // pianos
        8..=15 => Instrument {
            decay_s: 0.7,
            brightness: 18.0,
            ..PLUCKED(Waveform::Sine)
        }, // chromatic percussion
        16..=23 => Instrument {
            attack_s: 0.02,
            release_s: 0.15,
            transient: 0.05,
            body_decay_s: FOREVER,
            vibrato_depth: 0.0,
            ..SUSTAINED(Waveform::Square)
        }, // organs
        24..=31 => Instrument {
            transient: 0.45,
            ..PLUCKED(Waveform::Saw)
        }, // guitars
        32..=39 => Instrument {
            decay_s: 1.0,
            cutoff_ratio: 2.0,
            brightness: 5.0,
            ..PLUCKED(Waveform::Triangle)
        }, // basses
        40..=47 => Instrument {
            attack_s: 0.12,
            vibrato_depth: 0.006,
            ..SUSTAINED(Waveform::Saw)
        }, // solo strings
        48..=55 => Instrument {
            attack_s: 0.18,
            brightness: 4.5,
            ..SUSTAINED(Waveform::Saw)
        }, // ensembles
        56..=63 => Instrument {
            attack_s: 0.05,
            brightness: 9.0,
            transient: 0.2,
            ..SUSTAINED(Waveform::Saw)
        }, // brass
        64..=67 => Instrument {
            attack_s: 0.045,
            sustain_level: 0.8,
            cutoff_ratio: 2.0,
            brightness: 2.5,
            brightness_decay_s: 0.35,
            transient: 0.3,
            transient_s: 0.06,
            vibrato_depth: 0.005,
            ..SUSTAINED(Waveform::Saw)
        }, // saxophones: reeds are dominated by their low harmonics
        68..=70 => Instrument {
            attack_s: 0.04,
            cutoff_ratio: 1.8,
            brightness: 3.0,
            brightness_decay_s: 0.3,
            transient: 0.25,
            ..SUSTAINED(Waveform::Saw)
        }, // oboe, english horn, bassoon
        71 => Instrument {
            attack_s: 0.05,
            cutoff_ratio: 1.5,
            brightness: 1.5,
            brightness_decay_s: 0.4,
            transient: 0.2,
            transient_s: 0.07,
            vibrato_depth: 0.002,
            ..SUSTAINED(Waveform::Square)
        }, // clarinet: hollow, odd harmonics only
        72..=79 => Instrument {
            brightness: 4.0,
            transient: 0.3,
            transient_s: 0.09,
            ..SUSTAINED(Waveform::Sine)
        }, // pipes and flutes
        _ => SUSTAINED(Waveform::Square),     // synth leads and everything else
    }
}
