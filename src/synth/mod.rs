mod envelope;
mod filter;
mod instrument;
mod oscillator;
mod reverb;
mod rng;
mod voice;

pub use instrument::family_name;
use reverb::Reverb;
use voice::Voice;

use crate::engine::{Command, Engine, write_frame};

const VOICE_COUNT: usize = 8192;
const CHANNEL_COUNT: usize = 16;
const MASTER_GAIN: f32 = 0.3;
/// How much reverb is folded back in; enough to place the notes in a room.
const REVERB_MIX: f32 = 0.22;

pub struct Synth {
    voices: Vec<Voice>,
    /// Indices of the voices currently sounding, and of those free to claim.
    /// The mixer walks only the sounding ones, so the pool can be large without
    /// costing anything while the music is sparse.
    sounding: Vec<usize>,
    free: Vec<usize>,
    programs: [u8; CHANNEL_COUNT],
    sustain: [bool; CHANNEL_COUNT],
    volume: [f32; CHANNEL_COUNT],
    reverb: Reverb,
}

impl Synth {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            voices: (0..VOICE_COUNT)
                .map(|i| Voice::new(sample_rate, 0x9e37_79b9u32.wrapping_mul(i as u32 + 1)))
                .collect(),
            sounding: Vec::with_capacity(VOICE_COUNT),
            free: (0..VOICE_COUNT).rev().collect(),
            programs: [0; CHANNEL_COUNT],
            sustain: [false; CHANNEL_COUNT],
            volume: [1.0; CHANNEL_COUNT],
            reverb: Reverb::new(sample_rate),
        }
    }

    fn allocate_voice(&mut self) -> &mut Voice {
        let index = match self.free.pop() {
            Some(index) => {
                self.sounding.push(index);
                index
            }
            None => self.sounding[self.steal()],
        };
        &mut self.voices[index]
    }

    /// Which sounding voice to sacrifice when the pool is full. Prefer the
    /// quietest one that has had time to be heard: a voice that started moments
    /// ago is quiet *because* it is new, and taking it would cut off the note
    /// being played right now.
    fn steal(&self) -> usize {
        let quietest = self
            .sounding
            .iter()
            .enumerate()
            .filter(|&(_, &v)| self.voices[v].is_established())
            .min_by(|&(_, &a), &(_, &b)| self.voices[a].level().total_cmp(&self.voices[b].level()));
        if let Some((position, _)) = quietest {
            return position;
        }
        // Every voice is brand new, so fall back to the one that started first.
        self.sounding
            .iter()
            .enumerate()
            .max_by_key(|&(_, &v)| self.voices[v].age())
            .map_or(0, |(position, _)| position)
    }

    fn next_frame(&mut self) -> (f32, f32) {
        let mut dry = 0.0;
        let mut position = 0;
        while position < self.sounding.len() {
            let index = self.sounding[position];
            let voice = &mut self.voices[index];
            if voice.is_active() {
                let sample = voice.next_sample();
                let channel = voice.channel() as usize;
                dry += sample * self.volume[channel];
                position += 1;
            } else {
                self.sounding.swap_remove(position);
                self.free.push(index);
            }
        }
        let dry = dry * MASTER_GAIN;
        let (left, right) = self.reverb.process(dry);
        (dry + left * REVERB_MIX, dry + right * REVERB_MIX)
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
                let instrument = instrument::for_note(channel, program, note);
                self.allocate_voice()
                    .note_on(channel, note, velocity, instrument);
            }
            Command::NoteOff { channel, note } => {
                let held = self.sustain[channel as usize];
                for &index in &self.sounding {
                    let voice = &mut self.voices[index];
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
                    for &index in &self.sounding {
                        let voice = &mut self.voices[index];
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
                while let Some(index) = self.sounding.pop() {
                    self.voices[index].kill();
                    self.free.push(index);
                }
            }
        }
    }

    fn fill(&mut self, data: &mut [f32], channels: usize) {
        for frame in data.chunks_mut(channels) {
            let (left, right) = self.next_frame();
            write_frame(frame, left, right);
        }
    }
}
