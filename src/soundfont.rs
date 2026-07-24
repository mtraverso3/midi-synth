use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use rustysynth::{Synthesizer, SynthesizerSettings};

use crate::engine::{Command, Engine, write_frame};

const PROGRAM_CHANGE: i32 = 0xC0;
const CONTROL_CHANGE: i32 = 0xB0;
const CC_VOLUME: i32 = 7;
const CC_SUSTAIN: i32 = 64;

type Error = Box<dyn std::error::Error>;

#[derive(Clone)]
pub struct SoundFont(Arc<rustysynth::SoundFont>);

impl SoundFont {
    pub fn load(path: &Path) -> Result<Self, Error> {
        let mut file = File::open(path)?;
        Ok(Self(Arc::new(rustysynth::SoundFont::new(&mut file)?)))
    }
}

pub struct SoundFontEngine {
    synth: Synthesizer,
    left: Vec<f32>,
    right: Vec<f32>,
}

impl SoundFontEngine {
    pub fn new(bank: &SoundFont, sample_rate: u32) -> Result<Self, Error> {
        let settings = SynthesizerSettings::new(sample_rate as i32);
        Ok(Self {
            synth: Synthesizer::new(&bank.0, &settings)?,
            left: Vec::new(),
            right: Vec::new(),
        })
    }

    fn midi(&mut self, channel: u8, status: i32, data1: i32, data2: i32) {
        self.synth
            .process_midi_message(channel as i32, status, data1, data2);
    }
}

impl Engine for SoundFontEngine {
    fn handle(&mut self, command: Command) {
        match command {
            Command::NoteOn {
                channel,
                note,
                velocity,
            } => self
                .synth
                .note_on(channel as i32, note as i32, velocity as i32),
            Command::NoteOff { channel, note } => self.synth.note_off(channel as i32, note as i32),
            Command::ProgramChange { channel, program } => {
                self.midi(channel, PROGRAM_CHANGE, program as i32, 0)
            }
            Command::Sustain { channel, on } => self.midi(
                channel,
                CONTROL_CHANGE,
                CC_SUSTAIN,
                if on { 127 } else { 0 },
            ),
            Command::SetVolume { channel, level } => {
                self.midi(channel, CONTROL_CHANGE, CC_VOLUME, level as i32)
            }
            Command::AllNotesOff => self.synth.note_off_all(true),
            // Intercepted by the pause gate in `engine`.
            Command::SetPaused(_) => {}
        }
    }

    fn fill(&mut self, data: &mut [f32], channels: usize) {
        if channels == 0 {
            return;
        }
        let frames = data.len() / channels;
        self.left.resize(frames, 0.0);
        self.right.resize(frames, 0.0);
        self.synth.render(&mut self.left, &mut self.right);

        for (i, frame) in data.chunks_mut(channels).enumerate() {
            write_frame(frame, self.left[i], self.right[i]);
        }
    }
}
