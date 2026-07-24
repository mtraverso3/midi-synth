use crate::soundfont::{SoundFont, SoundFontEngine};
use crate::synth::Synth;

type Error = Box<dyn std::error::Error>;

pub enum Command {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8 },
    ProgramChange { channel: u8, program: u8 },
    Sustain { channel: u8, on: bool },
    SetVolume { channel: u8, level: u8 },
    SetPaused(bool),
    AllNotesOff,
}

pub trait Engine: Send {
    fn handle(&mut self, command: Command);
    /// Fill `data` with interleaved frames of `channels` samples each.
    fn fill(&mut self, data: &mut [f32], channels: usize);
}

pub fn build(soundfont: Option<SoundFont>, sample_rate: u32) -> Result<Box<dyn Engine>, Error> {
    let source: Box<dyn Engine> = match soundfont {
        Some(bank) => Box::new(SoundFontEngine::new(&bank, sample_rate)?),
        None => Box::new(Synth::new(sample_rate as f32)),
    };
    Ok(Box::new(Paused {
        source,
        paused: false,
    }))
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
