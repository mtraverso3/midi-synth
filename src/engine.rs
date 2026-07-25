use crate::limiter::Limiter;
use crate::soundfont::{SoundFont, SoundFontEngine};
use crate::synth::Synth;

type Error = Box<dyn std::error::Error>;

pub enum Command {
    NoteOn {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    NoteOff {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    ProgramChange {
        channel: u8,
        program: u8,
    },
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
    },
    /// Offset from centre in raw 14-bit units.
    PitchBend {
        channel: u8,
        offset: i16,
    },
    ChannelPressure {
        channel: u8,
        pressure: u8,
    },
    PolyPressure {
        channel: u8,
        note: u8,
        pressure: u8,
    },
    /// Universal master volume, 0.0..=1.0.
    SetMasterVolume(f32),
    /// Universal master balance, -1.0 hard left to 1.0 hard right.
    SetMasterBalance(f32),
    /// Universal master tuning, in semitones.
    SetMasterTuning(f32),
    /// Reset every channel to General MIDI defaults.
    Reset,
    SetPaused(bool),
    /// Silence everything on every channel, for seeking.
    AllNotesOff,
}

pub trait Engine: Send {
    fn handle(&mut self, command: Command);
    /// Fill `data` with interleaved frames of `channels` samples each.
    fn fill(&mut self, data: &mut [f32], channels: usize);
}

pub fn build(
    soundfont: Option<SoundFont>,
    sample_rate: u32,
    gain: f32,
) -> Result<Box<dyn Engine>, Error> {
    let source: Box<dyn Engine> = match soundfont {
        Some(bank) => Box::new(SoundFontEngine::new(&bank, sample_rate)?),
        None => Box::new(Synth::new(sample_rate as f32)),
    };
    let master = Master {
        source,
        gain,
        balance: (1.0, 1.0),
        limiter: Limiter::new(sample_rate),
    };
    Ok(Box::new(Paused {
        source: Box::new(master),
        paused: false,
    }))
}

/// Master bus: the listener's gain trim, then the limiter that keeps dense
/// music under the ceiling. Both engines pass through it.
struct Master {
    source: Box<dyn Engine>,
    gain: f32,
    balance: (f32, f32),
    limiter: Limiter,
}

impl Engine for Master {
    fn handle(&mut self, command: Command) {
        match command {
            Command::SetMasterBalance(position) => {
                self.balance = crate::midi::stereo_gains(position)
            }
            Command::Reset => {
                self.balance = (1.0, 1.0);
                self.source.handle(command);
            }
            other => self.source.handle(other),
        }
    }

    fn fill(&mut self, data: &mut [f32], channels: usize) {
        self.source.fill(data, channels);
        let (left, right) = self.balance;
        if self.gain != 1.0 || self.balance != (1.0, 1.0) {
            for frame in data.chunks_mut(channels) {
                match frame {
                    [mono] => *mono *= self.gain,
                    [l, r, ..] => {
                        *l *= self.gain * left;
                        *r *= self.gain * right;
                    }
                    [] => {}
                }
            }
        }
        self.limiter.process(data, channels);
    }
}

pub(crate) fn write_frame(frame: &mut [f32], left: f32, right: f32) {
    match frame {
        [mono] => *mono = 0.5 * (left + right),
        [l, r, rest @ ..] => {
            *l = left;
            *r = right;
            rest.fill(0.0);
        }
        [] => {}
    }
}

/// While paused the source is never advanced, so held notes resume exactly
/// where they stopped.
struct Paused {
    source: Box<dyn Engine>,
    paused: bool,
}

impl Engine for Paused {
    fn handle(&mut self, command: Command) {
        match command {
            Command::SetPaused(paused) => self.paused = paused,
            other => self.source.handle(other),
        }
    }

    fn fill(&mut self, data: &mut [f32], channels: usize) {
        if self.paused {
            data.fill(0.0);
        } else {
            self.source.fill(data, channels);
        }
    }
}
