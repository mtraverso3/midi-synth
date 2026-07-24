mod envelope;
mod oscillator;
mod voice;

use voice::Voice;

const VOICE_COUNT: usize = 16;
const MASTER_GAIN: f32 = 0.3;

pub enum SynthCommand {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
}

pub struct Synth {
    voices: Vec<Voice>,
}

impl Synth {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            voices: (0..VOICE_COUNT).map(|_| Voice::new(sample_rate)).collect(),
        }
    }

    pub fn handle(&mut self, command: SynthCommand) {
        match command {
            SynthCommand::NoteOn { note, velocity } => {
                self.allocate_voice().note_on(note, velocity);
            }
            SynthCommand::NoteOff { note } => {
                for voice in &mut self.voices {
                    if voice.note() == Some(note) && voice.is_active() {
                        voice.note_off();
                    }
                }
            }
        }
    }

    fn allocate_voice(&mut self) -> &mut Voice {
        let index = self.voices.iter().position(|v| !v.is_active()).unwrap_or(0);
        &mut self.voices[index]
    }

    pub fn next_sample(&mut self) -> f32 {
        let mixed: f32 = self
            .voices
            .iter_mut()
            .filter(|v| v.is_active())
            .map(|v| v.next_sample())
            .sum();
        (mixed * MASTER_GAIN).clamp(-1.0, 1.0)
    }
}
