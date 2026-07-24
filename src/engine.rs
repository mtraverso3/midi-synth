use std::sync::Arc;

use rustysynth::SoundFont;

use crate::soundfont::SoundFontEngine;
use crate::synth::{Synth, SynthCommand};

/// A sound source driven by [`SynthCommand`]s. Either our built-in synth or a
/// SoundFont-backed one; the audio path treats them the same.
pub trait Engine: Send {
    fn handle(&mut self, command: SynthCommand);
    /// Fill an interleaved buffer with `channels` channels per frame.
    fn fill(&mut self, data: &mut [f32], channels: usize);
}

/// Build the SoundFont engine if one is supplied, else the built-in synth.
pub fn build(soundfont: Option<Arc<SoundFont>>, sample_rate: u32) -> Box<dyn Engine> {
    match soundfont {
        Some(sf) => Box::new(SoundFontEngine::new(sf, sample_rate)),
        None => Box::new(Synth::new(sample_rate as f32)),
    }
}

impl Engine for Synth {
    fn handle(&mut self, command: SynthCommand) {
        Synth::handle(self, command);
    }

    fn fill(&mut self, data: &mut [f32], channels: usize) {
        for frame in data.chunks_mut(channels) {
            let sample = self.next_sample();
            frame.fill(sample);
        }
    }
}
