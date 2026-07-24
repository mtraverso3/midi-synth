use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};

use crate::engine::Engine;
use crate::synth::SynthCommand;

const CMD_PROGRAM_CHANGE: i32 = 0xC0;
const CMD_CONTROL_CHANGE: i32 = 0xB0;
const CC_VOLUME: i32 = 7;
const CC_SUSTAIN: i32 = 64;

type Error = Box<dyn std::error::Error>;

pub fn load(path: &Path) -> Result<Arc<SoundFont>, Error> {
    let mut file = File::open(path)?;
    Ok(Arc::new(SoundFont::new(&mut file)?))
}

/// Wraps a rustysynth [`Synthesizer`], which renders real instrument samples
/// from the SoundFont in stereo blocks.
pub struct SoundFontEngine {
    synth: Synthesizer,
    left: Vec<f32>,
    right: Vec<f32>,
    paused: bool,
}

impl SoundFontEngine {
    pub fn new(soundfont: Arc<SoundFont>, sample_rate: u32) -> Self {
        let settings = SynthesizerSettings::new(sample_rate as i32);
        let synth = Synthesizer::new(&soundfont, &settings).expect("invalid synthesizer settings");
        Self {
            synth,
            left: Vec::new(),
            right: Vec::new(),
            paused: false,
        }
    }
}

impl Engine for SoundFontEngine {
    fn handle(&mut self, command: SynthCommand) {
        match command {
            SynthCommand::NoteOn {
                channel,
                note,
                velocity,
            } => self
                .synth
                .note_on(channel as i32, note as i32, velocity as i32),
            SynthCommand::NoteOff { channel, note } => {
                self.synth.note_off(channel as i32, note as i32)
            }
            SynthCommand::ProgramChange { channel, program } => self.synth.process_midi_message(
                channel as i32,
                CMD_PROGRAM_CHANGE,
                program as i32,
                0,
            ),
            SynthCommand::Sustain { channel, on } => self.synth.process_midi_message(
                channel as i32,
                CMD_CONTROL_CHANGE,
                CC_SUSTAIN,
                if on { 127 } else { 0 },
            ),
            SynthCommand::SetVolume { channel, level } => self.synth.process_midi_message(
                channel as i32,
                CMD_CONTROL_CHANGE,
                CC_VOLUME,
                level as i32,
            ),
            SynthCommand::SetPaused(paused) => self.paused = paused,
            SynthCommand::AllNotesOff => self.synth.note_off_all(true),
        }
    }

    fn fill(&mut self, data: &mut [f32], channels: usize) {
        if self.paused {
            data.fill(0.0);
            return;
        }
        let frames = data.len() / channels.max(1);
        self.left.resize(frames, 0.0);
        self.right.resize(frames, 0.0);
        self.synth.render(&mut self.left, &mut self.right);

        for (i, frame) in data.chunks_mut(channels).enumerate() {
            match frame {
                [mono] => *mono = 0.5 * (self.left[i] + self.right[i]),
                [l, r, rest @ ..] => {
                    *l = self.left[i];
                    *r = self.right[i];
                    rest.fill(self.left[i]);
                }
                [] => {}
            }
        }
    }
}
