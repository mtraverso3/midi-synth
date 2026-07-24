use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use rustysynth::{Synthesizer, SynthesizerSettings};

use crate::engine::{Command, Engine, write_frame};
use crate::midi::PITCH_BEND_CENTRE;

const POLY_PRESSURE: i32 = 0xA0;
const CONTROL_CHANGE: i32 = 0xB0;
const PROGRAM_CHANGE: i32 = 0xC0;
const CHANNEL_PRESSURE: i32 = 0xD0;
const PITCH_BEND: i32 = 0xE0;

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
                .note_on(i32::from(channel), i32::from(note), i32::from(velocity)),
            Command::NoteOff { channel, note, .. } => {
                self.synth.note_off(i32::from(channel), i32::from(note));
            }
            Command::ProgramChange { channel, program } => {
                self.midi(channel, PROGRAM_CHANGE, i32::from(program), 0);
            }
            // rustysynth is a full General MIDI synth, so controllers, bend and
            // pressure go through untouched rather than being reinterpreted.
            Command::ControlChange {
                channel,
                controller,
                value,
            } => self.midi(
                channel,
                CONTROL_CHANGE,
                i32::from(controller),
                i32::from(value),
            ),
            Command::PitchBend { channel, offset } => {
                let raw = (offset + PITCH_BEND_CENTRE).clamp(0, 16383);
                self.midi(
                    channel,
                    PITCH_BEND,
                    i32::from(raw & 0x7f),
                    i32::from(raw >> 7),
                );
            }
            Command::ChannelPressure { channel, pressure } => {
                self.midi(channel, CHANNEL_PRESSURE, i32::from(pressure), 0);
            }
            Command::PolyPressure {
                channel,
                note,
                pressure,
            } => self.midi(channel, POLY_PRESSURE, i32::from(note), i32::from(pressure)),
            Command::SetMasterVolume(level) => self.synth.set_master_volume(level),
            Command::Reset => self.synth.reset(),
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
