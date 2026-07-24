mod envelope;
mod filter;
mod instrument;
mod oscillator;
mod rng;
mod voice;

pub use instrument::family_name;
use voice::Voice;

use crate::engine::{Command, Engine, write_frame};

const VOICE_COUNT: usize = 256;
const CHANNEL_COUNT: usize = 16;
const MASTER_GAIN: f32 = 0.3;

pub struct Synth {
    voices: Vec<Voice>,
    programs: [u8; CHANNEL_COUNT],
    sustain: [bool; CHANNEL_COUNT],
    volume: [f32; CHANNEL_COUNT],
}

impl Synth {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            voices: (0..VOICE_COUNT)
                .map(|i| Voice::new(sample_rate, 0x9e37_79b9u32.wrapping_mul(i as u32 + 1)))
                .collect(),
            programs: [0; CHANNEL_COUNT],
            sustain: [false; CHANNEL_COUNT],
            volume: [1.0; CHANNEL_COUNT],
        }
    }

    fn allocate_voice(&mut self) -> &mut Voice {
        // Prefer an idle voice; if all are busy, steal the quietest one so the
        // interruption is as inaudible as possible.
        if let Some(index) = self.voices.iter().position(|v| !v.is_active()) {
            return &mut self.voices[index];
        }
        let index = self
            .voices
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.level().total_cmp(&b.level()))
            .map(|(i, _)| i)
            .unwrap_or(0);
        &mut self.voices[index]
    }

    fn next_sample(&mut self) -> f32 {
        let volume = &self.volume;
        let mixed: f32 = self
            .voices
            .iter_mut()
            .filter(|v| v.is_active())
            .map(|v| v.next_sample() * volume[v.channel() as usize])
            .sum();
        (mixed * MASTER_GAIN).clamp(-1.0, 1.0)
    }
}

impl Engine for Synth {
    fn handle(&mut self, command: Command) {
        match command {
            Command::NoteOn {
                channel,
                note,
                velocity,
            } => {
                let program = self.programs[channel as usize];
                let instrument = instrument::for_channel(channel, program);
                self.allocate_voice()
                    .note_on(channel, note, velocity, instrument);
            }
            Command::NoteOff { channel, note } => {
                let held = self.sustain[channel as usize];
                for voice in &mut self.voices {
                    if voice.is_active() && voice.matches(channel, note) {
                        if held {
                            voice.hold();
                        } else {
                            voice.note_off();
                        }
                    }
                }
            }
            Command::ProgramChange { channel, program } => {
                self.programs[channel as usize] = program;
            }
            Command::Sustain { channel, on } => {
                self.sustain[channel as usize] = on;
                if !on {
                    // Pedal up: release every voice it was holding on this channel.
                    for voice in &mut self.voices {
                        if voice.channel() == channel && voice.is_sustained() {
                            voice.note_off();
                        }
                    }
                }
            }
            Command::SetVolume { channel, level } => {
                self.volume[channel as usize] = level as f32 / 127.0;
            }
            // Intercepted by the pause gate in `engine`.
            Command::SetPaused(_) => {}
            Command::AllNotesOff => {
                for voice in &mut self.voices {
                    voice.kill();
                }
            }
        }
    }

    fn fill(&mut self, data: &mut [f32], channels: usize) {
        for frame in data.chunks_mut(channels) {
            let sample = self.next_sample();
            write_frame(frame, sample, sample);
        }
    }
}
