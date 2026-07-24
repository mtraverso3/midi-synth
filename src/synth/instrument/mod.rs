mod melodic;
mod percussion;

use super::oscillator::{SQUARE, Waveform};

/// General MIDI reserves channel 9 for percussion, regardless of program.
const DRUM_CHANNEL: u8 = 9;

/// How a voice is shaped, from oscillator through to amplifier. Every General
/// MIDI instrument is one of these: the archetypes below are the starting
/// points, and each program overrides only what makes it distinctive.
#[derive(Clone, Copy)]
pub struct Instrument {
    pub waveform: Waveform,
    pub attack_s: f32,
    pub decay_s: f32,
    pub sustain_level: f32,
    pub release_s: f32,
    /// Cutoff once the filter envelope has closed, as a multiple of the pitch.
    pub cutoff_ratio: f32,
    /// Extra cutoff at the peak of the filter envelope, as a further multiple.
    pub brightness: f32,
    /// How fast the filter envelope closes, darkening the note as it rings.
    pub brightness_decay_s: f32,
    /// Inharmonic noise burst at the attack: hammers, breath, bow scratch.
    pub transient: f32,
    pub transient_s: f32,
    /// Noise mixed through the body of the note, for breathy and airy voices.
    pub noise_mix: f32,
    /// Time constant for a held note losing energy; large means it holds forever.
    pub body_decay_s: f32,
    pub vibrato_depth: f32,
    /// Semitones the pitch starts above its target, falling away at the attack.
    /// Negative scoops up into the note instead.
    pub pitch_drop: f32,
    pub pitch_drop_s: f32,
    /// Pitch in Hz for voices where the note picks the sound, not the pitch.
    pub fixed_pitch: Option<f32>,
    /// Filter resonance, relative to the patch's own. Sound controller 71 moves it.
    pub resonance: f32,
    /// Output trim, for balancing one instrument against the rest of the set.
    pub level: f32,
}

pub const FOREVER: f32 = 1.0e9;

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
    noise_mix: 0.0,
    body_decay_s: FOREVER,
    vibrato_depth: 0.0,
    pitch_drop: 0.0,
    pitch_drop_s: 0.02,
    fixed_pitch: None,
    resonance: 1.0,
    level: 1.0,
};

/// Hammered or mallet-struck: instant onset, then a long ring down to silence.
pub const STRUCK: Instrument = Instrument {
    attack_s: 0.002,
    decay_s: 1.4,
    release_s: 0.25,
    cutoff_ratio: 2.5,
    brightness: 14.0,
    brightness_decay_s: 0.25,
    transient: 0.35,
    transient_s: 0.012,
    ..BASE
};

/// Plucked string: sharper and shorter than struck, with louder finger noise.
pub const PLUCKED: Instrument = Instrument {
    attack_s: 0.001,
    decay_s: 0.9,
    release_s: 0.2,
    cutoff_ratio: 2.2,
    brightness: 12.0,
    brightness_decay_s: 0.2,
    transient: 0.45,
    transient_s: 0.01,
    ..BASE
};

/// Bowed string: eases in, sustains under vibrato, bow noise through the body.
pub const BOWED: Instrument = Instrument {
    waveform: Waveform::Saw,
    attack_s: 0.12,
    decay_s: 0.3,
    sustain_level: 0.8,
    release_s: 0.3,
    cutoff_ratio: 2.5,
    brightness: 4.0,
    brightness_decay_s: 0.8,
    transient: 0.15,
    transient_s: 0.08,
    noise_mix: 0.04,
    body_decay_s: 8.0,
    vibrato_depth: 0.006,
    ..BASE
};

/// Blown edge tone: soft, airy, and carried by its breath noise.
pub const BLOWN: Instrument = Instrument {
    waveform: Waveform::Sine,
    attack_s: 0.06,
    decay_s: 0.2,
    sustain_level: 0.85,
    release_s: 0.2,
    cutoff_ratio: 2.5,
    brightness: 3.0,
    brightness_decay_s: 0.6,
    transient: 0.3,
    transient_s: 0.09,
    noise_mix: 0.1,
    body_decay_s: 12.0,
    vibrato_depth: 0.004,
    ..BASE
};

/// Reed: rich but low-ordered harmonics, so it stays warm rather than buzzy.
pub const REED: Instrument = Instrument {
    waveform: Waveform::Saw,
    attack_s: 0.045,
    decay_s: 0.25,
    sustain_level: 0.8,
    release_s: 0.3,
    cutoff_ratio: 2.0,
    brightness: 2.5,
    brightness_decay_s: 0.35,
    transient: 0.3,
    transient_s: 0.06,
    noise_mix: 0.05,
    body_decay_s: 8.0,
    vibrato_depth: 0.005,
    ..BASE
};

/// Brass: scoops into the note and brightens as it is pushed.
pub const BRASS: Instrument = Instrument {
    waveform: Waveform::Saw,
    attack_s: 0.05,
    decay_s: 0.25,
    sustain_level: 0.8,
    release_s: 0.25,
    cutoff_ratio: 1.8,
    brightness: 7.0,
    brightness_decay_s: 0.5,
    transient: 0.2,
    transient_s: 0.04,
    noise_mix: 0.03,
    body_decay_s: 8.0,
    vibrato_depth: 0.003,
    pitch_drop: -0.3,
    pitch_drop_s: 0.04,
    ..BASE
};

/// Organ: no dynamics of its own. On, flat, then off.
pub const ORGAN: Instrument = Instrument {
    waveform: SQUARE,
    attack_s: 0.015,
    decay_s: 0.05,
    sustain_level: 1.0,
    release_s: 0.08,
    cutoff_ratio: 3.0,
    brightness: 2.5,
    brightness_decay_s: 1.0,
    transient: 0.05,
    ..BASE
};

/// Pad: slow in, slow out, never in a hurry.
pub const PAD: Instrument = Instrument {
    waveform: Waveform::Saw,
    attack_s: 0.5,
    decay_s: 0.6,
    sustain_level: 0.8,
    release_s: 0.9,
    cutoff_ratio: 2.0,
    brightness: 3.0,
    brightness_decay_s: 2.0,
    vibrato_depth: 0.003,
    ..BASE
};

/// Synth lead: deliberately electronic, which is the point.
pub const LEAD: Instrument = Instrument {
    waveform: SQUARE,
    attack_s: 0.01,
    decay_s: 0.2,
    sustain_level: 0.85,
    release_s: 0.15,
    cutoff_ratio: 2.5,
    brightness: 6.0,
    brightness_decay_s: 0.6,
    transient: 0.1,
    vibrato_depth: 0.004,
    ..BASE
};

/// Bell and metal: struck, very bright at the onset, ringing for a long time.
pub const BELL: Instrument = Instrument {
    waveform: Waveform::Sine,
    attack_s: 0.001,
    decay_s: 2.5,
    release_s: 1.0,
    cutoff_ratio: 4.0,
    brightness: 18.0,
    brightness_decay_s: 0.15,
    transient: 0.5,
    transient_s: 0.02,
    ..BASE
};

/// Struck skin or shell: a short thud, tuned by `fixed_pitch` rather than note.
pub const HIT: Instrument = Instrument {
    waveform: Waveform::Noise,
    attack_s: 0.0005,
    decay_s: 0.12,
    release_s: 0.05,
    cutoff_ratio: 1.0,
    brightness: 5.0,
    brightness_decay_s: 0.06,
    fixed_pitch: Some(400.0),
    ..BASE
};

/// The voice for one note: percussion is picked by note, everything else by program.
pub fn for_note(channel: u8, program: u8, note: u8) -> Instrument {
    if channel == DRUM_CHANNEL {
        percussion::voice(note)
    } else {
        melodic::voice(program)
    }
}

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
        96..=103 => "Effects",
        104..=111 => "Ethnic",
        112..=119 => "Percussive",
        _ => "SFX",
    }
}
