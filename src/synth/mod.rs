mod channel;
mod envelope;
mod filter;
mod instrument;
mod oscillator;
mod reverb;
mod rng;
mod voice;

use channel::{Action, Channel};
pub use instrument::family_name;
use reverb::Reverb;
use voice::Voice;

use crate::engine::{Command, Engine, write_frame};

const VOICE_COUNT: usize = 8192;
const CHANNEL_COUNT: usize = 16;
const MASTER_GAIN: f32 = 0.42;
/// Vibrato that full polyphonic key pressure adds to a single note.
const POLY_PRESSURE_DEPTH: f32 = 0.02;
/// How much reverb is folded back in; enough to place the notes in a room.
const REVERB_MIX: f32 = 0.22;

pub struct Synth {
    voices: Vec<Voice>,
    /// Indices of the voices currently sounding, and of those free to claim.
    /// The mixer walks only the sounding ones, so the pool can be large without
    /// costing anything while the music is sparse.
    sounding: Vec<usize>,
    free: Vec<usize>,
    channels: [Channel; CHANNEL_COUNT],
    master_volume: f32,
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
            channels: std::array::from_fn(|_| Channel::default()),
            master_volume: 1.0,
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

    /// Carry out whatever a controller asked of the sounding voices.
    fn apply(&mut self, channel: u8, action: Action) {
        match action {
            Action::None => {}
            Action::ModulationChanged => {}
            Action::PedalReleased => {
                let state = &self.channels[channel as usize];
                let held = state.sustain || state.sostenuto;
                if !held {
                    for &index in &self.sounding {
                        let voice = &mut self.voices[index];
                        if voice.channel() == channel && voice.is_sustained() {
                            voice.note_off();
                        }
                    }
                }
            }
            Action::AllNotesOff => {
                for &index in &self.sounding {
                    let voice = &mut self.voices[index];
                    if voice.channel() == channel {
                        voice.note_off();
                    }
                }
            }
            Action::AllSoundOff => {
                let mut position = 0;
                while position < self.sounding.len() {
                    let index = self.sounding[position];
                    if self.voices[index].channel() == channel {
                        self.voices[index].kill();
                        self.sounding.swap_remove(position);
                        self.free.push(index);
                    } else {
                        position += 1;
                    }
                }
            }
        }
    }

    fn next_frame(&mut self) -> (f32, f32) {
        let mut dry_left = 0.0;
        let mut dry_right = 0.0;
        let mut position = 0;
        while position < self.sounding.len() {
            let index = self.sounding[position];
            let voice = &mut self.voices[index];
            if voice.is_active() {
                let state = &self.channels[voice.channel() as usize];
                let sample = voice.next_sample(state.pitch_scale, state.vibrato());
                let level = sample * state.level();
                let (left, right) = state.pan;
                dry_left += level * left;
                dry_right += level * right;
                position += 1;
            } else {
                self.sounding.swap_remove(position);
                self.free.push(index);
            }
        }
        let gain = MASTER_GAIN * self.master_volume;
        let dry_left = dry_left * gain;
        let dry_right = dry_right * gain;
        let (wet_left, wet_right) = self.reverb.process((dry_left + dry_right) * 0.5);
        (
            dry_left + wet_left * REVERB_MIX,
            dry_right + wet_right * REVERB_MIX,
        )
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
                let state = &self.channels[channel as usize];
                let mut instrument = instrument::for_note(channel, state.program, note);
                instrument.attack_s *= state.attack_scale;
                instrument.release_s *= state.release_scale;
                instrument.brightness *= state.brightness_scale;
                instrument.resonance = state.resonance;
                let velocity = (f32::from(velocity) * state.soft) as u8;
                self.allocate_voice()
                    .note_on(channel, note, velocity, instrument);
            }
            Command::NoteOff { channel, note, .. } => {
                let state = &self.channels[channel as usize];
                let held = state.sustain || state.sostenuto;
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
                self.channels[channel as usize].program = program;
            }
            Command::ControlChange {
                channel,
                controller,
                value,
            } => {
                let action = self.channels[channel as usize].control(controller, value);
                self.apply(channel, action);
            }
            Command::PitchBend { channel, offset } => {
                self.channels[channel as usize].set_bend(offset);
            }
            Command::ChannelPressure { channel, pressure } => {
                self.channels[channel as usize].pressure = f32::from(pressure) / 127.0;
            }
            Command::PolyPressure {
                channel,
                note,
                pressure,
            } => {
                let depth = f32::from(pressure) / 127.0 * POLY_PRESSURE_DEPTH;
                for &index in &self.sounding {
                    let voice = &mut self.voices[index];
                    if voice.is_active() && voice.matches(channel, note) {
                        voice.set_pressure(depth);
                    }
                }
            }
            Command::SetMasterVolume(level) => self.master_volume = level,
            Command::Reset => {
                self.channels = std::array::from_fn(|_| Channel::default());
                self.master_volume = 1.0;
                while let Some(index) = self.sounding.pop() {
                    self.voices[index].kill();
                    self.free.push(index);
                }
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
