mod envelope;
mod filter;
mod instrument;
mod oscillator;
mod voice;

use voice::Voice;

const VOICE_COUNT: usize = 64;
const CHANNEL_COUNT: usize = 16;
const MASTER_GAIN: f32 = 0.3;

pub enum SynthCommand {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8 },
    ProgramChange { channel: u8, program: u8 },
}

pub struct Synth {
    voices: Vec<Voice>,
    programs: [u8; CHANNEL_COUNT],
}

impl Synth {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            voices: (0..VOICE_COUNT).map(|_| Voice::new(sample_rate)).collect(),
            programs: [0; CHANNEL_COUNT],
        }
    }

    pub fn handle(&mut self, command: SynthCommand) {
        match command {
            SynthCommand::NoteOn {
                channel,
                note,
                velocity,
            } => {
                let program = self.programs[channel as usize];
                let instrument = instrument::for_channel(channel, program);
                self.allocate_voice().note_on(channel, note, velocity, instrument);
            }
            SynthCommand::NoteOff { channel, note } => {
                for voice in &mut self.voices {
                    if voice.is_active() && voice.matches(channel, note) {
                        voice.note_off();
                    }
                }
            }
            SynthCommand::ProgramChange { channel, program } => {
                self.programs[channel as usize] = program;
            }
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
