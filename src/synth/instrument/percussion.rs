use super::super::oscillator::Waveform;
use super::{HIT, Instrument};

/// A drum kit, keyed by General MIDI note number: on channel 9 the note picks
/// the instrument rather than the pitch.
pub fn voice(note: u8) -> Instrument {
    match note {
        35 => kick(50.0, 0.22),
        36 => kick(55.0, 0.18),
        37 => stick(1400.0, 0.035), // side stick
        38 => snare(190.0, 0.16, 0.75),
        39 => Instrument {
            decay_s: 0.13,
            cutoff_ratio: 4.0,
            brightness: 3.0,
            fixed_pitch: Some(600.0),
            ..HIT
        }, // hand clap
        40 => snare(240.0, 0.13, 0.85),
        41 => tom(90.0),
        42 => hat(0.035),
        43 => tom(110.0),
        44 => hat(0.05), // pedal hi-hat
        45 => tom(135.0),
        46 => Instrument {
            decay_s: 0.32,
            cutoff_ratio: 9.0,
            brightness: 4.0,
            brightness_decay_s: 0.2,
            fixed_pitch: Some(900.0),
            ..HIT
        }, // open hi-hat
        47 => tom(165.0),
        48 => tom(200.0),
        49 => cymbal(1.6, 12.0), // crash 1
        50 => tom(240.0),
        51 => ride(1.0),         // ride 1
        52 => cymbal(1.8, 14.0), // chinese cymbal
        53 => Instrument {
            decay_s: 0.7,
            cutoff_ratio: 6.0,
            brightness: 8.0,
            transient: 0.4,
            fixed_pitch: Some(1100.0),
            waveform: Waveform::Triangle,
            ..HIT
        }, // ride bell
        54 => Instrument {
            decay_s: 0.2,
            cutoff_ratio: 11.0,
            brightness: 5.0,
            fixed_pitch: Some(1000.0),
            ..HIT
        }, // tambourine
        55 => cymbal(1.0, 13.0), // splash
        56 => Instrument {
            waveform: Waveform::Pulse(0.25),
            decay_s: 0.28,
            cutoff_ratio: 4.0,
            brightness: 6.0,
            transient: 0.4,
            fixed_pitch: Some(830.0),
            ..HIT
        }, // cowbell
        57 => cymbal(1.5, 11.0), // crash 2
        58 => Instrument {
            decay_s: 0.5,
            cutoff_ratio: 7.0,
            brightness: 5.0,
            fixed_pitch: Some(700.0),
            ..HIT
        }, // vibraslap
        59 => ride(0.9),         // ride 2
        60 => hand_drum(430.0),  // hi bongo
        61 => hand_drum(330.0),  // low bongo
        62 => hand_drum(280.0),  // mute hi conga
        63 => hand_drum(230.0),  // open hi conga
        64 => hand_drum(180.0),  // low conga
        65 => hand_drum(300.0),  // high timbale
        66 => hand_drum(240.0),  // low timbale
        67 => agogo(1300.0),
        68 => agogo(1050.0),
        69 => shaker(0.11, 14.0), // cabasa
        70 => shaker(0.08, 16.0), // maracas
        71 => whistle(0.15),      // short whistle
        72 => whistle(0.5),       // long whistle
        73 => guiro(0.12),
        74 => guiro(0.45),
        75 => stick(2200.0, 0.045), // claves
        76 => stick(1900.0, 0.05),  // hi wood block
        77 => stick(1500.0, 0.06),  // low wood block
        78 => cuica(0.16, 620.0),   // mute cuica
        79 => cuica(0.4, 480.0),    // open cuica
        80 => triangle_bell(0.35),  // mute triangle
        81 => triangle_bell(1.6),   // open triangle
        _ => HIT,
    }
}

/// Bass drum: a low sine that drops in pitch as it is struck.
fn kick(pitch: f32, decay_s: f32) -> Instrument {
    Instrument {
        waveform: Waveform::Sine,
        decay_s,
        cutoff_ratio: 6.0,
        brightness: 3.0,
        brightness_decay_s: 0.03,
        transient: 0.35,
        transient_s: 0.006,
        pitch_drop: 14.0,
        pitch_drop_s: 0.03,
        fixed_pitch: Some(pitch),
        ..HIT
    }
}

/// Snare: a tuned shell under a wash of wire noise.
fn snare(pitch: f32, decay_s: f32, noise_mix: f32) -> Instrument {
    Instrument {
        waveform: Waveform::Triangle,
        decay_s,
        cutoff_ratio: 12.0,
        brightness: 4.0,
        brightness_decay_s: 0.05,
        transient: 0.3,
        noise_mix,
        pitch_drop: 3.0,
        pitch_drop_s: 0.02,
        fixed_pitch: Some(pitch),
        ..HIT
    }
}

fn tom(pitch: f32) -> Instrument {
    Instrument {
        waveform: Waveform::Sine,
        decay_s: 0.3,
        cutoff_ratio: 5.0,
        brightness: 3.5,
        brightness_decay_s: 0.06,
        transient: 0.35,
        noise_mix: 0.12,
        pitch_drop: 5.0,
        pitch_drop_s: 0.05,
        fixed_pitch: Some(pitch),
        ..HIT
    }
}

fn hat(decay_s: f32) -> Instrument {
    Instrument {
        decay_s,
        cutoff_ratio: 11.0,
        brightness: 4.0,
        brightness_decay_s: 0.02,
        fixed_pitch: Some(1000.0),
        ..HIT
    }
}

fn cymbal(decay_s: f32, cutoff_ratio: f32) -> Instrument {
    Instrument {
        decay_s,
        release_s: 0.4,
        cutoff_ratio,
        brightness: 3.0,
        brightness_decay_s: 0.5,
        fixed_pitch: Some(800.0),
        ..HIT
    }
}

fn ride(decay_s: f32) -> Instrument {
    Instrument {
        decay_s,
        cutoff_ratio: 10.0,
        brightness: 5.0,
        brightness_decay_s: 0.1,
        transient: 0.35,
        fixed_pitch: Some(900.0),
        ..HIT
    }
}

fn hand_drum(pitch: f32) -> Instrument {
    Instrument {
        waveform: Waveform::Triangle,
        decay_s: 0.16,
        cutoff_ratio: 4.0,
        brightness: 4.0,
        brightness_decay_s: 0.04,
        transient: 0.4,
        noise_mix: 0.2,
        pitch_drop: 2.0,
        fixed_pitch: Some(pitch),
        ..HIT
    }
}

fn agogo(pitch: f32) -> Instrument {
    Instrument {
        waveform: Waveform::Pulse(0.2),
        decay_s: 0.3,
        cutoff_ratio: 4.0,
        brightness: 7.0,
        transient: 0.4,
        fixed_pitch: Some(pitch),
        ..HIT
    }
}

fn shaker(decay_s: f32, cutoff_ratio: f32) -> Instrument {
    Instrument {
        decay_s,
        cutoff_ratio,
        brightness: 3.0,
        brightness_decay_s: 0.03,
        fixed_pitch: Some(1000.0),
        ..HIT
    }
}

/// Sharp wooden click: claves, wood blocks, rim.
fn stick(pitch: f32, decay_s: f32) -> Instrument {
    Instrument {
        waveform: Waveform::Triangle,
        decay_s,
        cutoff_ratio: 3.0,
        brightness: 8.0,
        brightness_decay_s: 0.01,
        transient: 0.5,
        transient_s: 0.004,
        fixed_pitch: Some(pitch),
        ..HIT
    }
}

fn whistle(decay_s: f32) -> Instrument {
    Instrument {
        waveform: Waveform::Sine,
        decay_s,
        cutoff_ratio: 4.0,
        brightness: 2.0,
        noise_mix: 0.15,
        vibrato_depth: 0.02,
        fixed_pitch: Some(2300.0),
        ..HIT
    }
}

fn guiro(decay_s: f32) -> Instrument {
    Instrument {
        decay_s,
        cutoff_ratio: 8.0,
        brightness: 3.0,
        brightness_decay_s: 0.15,
        fixed_pitch: Some(700.0),
        ..HIT
    }
}

fn cuica(decay_s: f32, pitch: f32) -> Instrument {
    Instrument {
        waveform: Waveform::Sine,
        decay_s,
        cutoff_ratio: 3.0,
        brightness: 4.0,
        noise_mix: 0.25,
        pitch_drop: -4.0,
        pitch_drop_s: 0.12,
        fixed_pitch: Some(pitch),
        ..HIT
    }
}

fn triangle_bell(decay_s: f32) -> Instrument {
    Instrument {
        waveform: Waveform::Sine,
        decay_s,
        release_s: 0.3,
        cutoff_ratio: 5.0,
        brightness: 9.0,
        brightness_decay_s: 0.1,
        transient: 0.45,
        fixed_pitch: Some(3200.0),
        ..HIT
    }
}
