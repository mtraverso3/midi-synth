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
use crate::midi::{NEUTRAL_RELEASE_VELOCITY, UNSPECIFIED_RELEASE_VELOCITY};

const VOICE_COUNT: usize = 8192;
const CHANNEL_COUNT: usize = 16;
const MASTER_GAIN: f32 = 0.53;
/// Vibrato that full polyphonic key pressure adds to a single note.
const POLY_PRESSURE_DEPTH: f32 = 0.02;
/// How much of the reverb send is folded back in. Scaled so a channel sitting
/// at the General MIDI default send lands where the fixed mix used to.
const REVERB_MIX: f32 = 0.7;

pub struct Synth {
    voices: Vec<Voice>,
    /// Indices of the voices currently sounding, and of those free to claim.
    /// The mixer walks only the sounding ones, so the pool can be large without
    /// costing anything while the music is sparse.
    sounding: Vec<usize>,
    free: Vec<usize>,
    channels: [Channel; CHANNEL_COUNT],
    master_volume: f32,
    master_tuning: f32,
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
            master_tuning: 1.0,
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
            Action::SustainReleased => self.release_held(channel),
            Action::SostenutoPressed => {
                for &index in &self.sounding {
                    let voice = &mut self.voices[index];
                    if voice.channel() == channel {
                        voice.capture();
                    }
                }
            }
            Action::SostenutoReleased | Action::ResetControllers => {
                let clear_pressure = matches!(action, Action::ResetControllers);
                for &index in &self.sounding {
                    let voice = &mut self.voices[index];
                    if voice.channel() == channel {
                        voice.free_sostenuto();
                        if clear_pressure {
                            voice.set_pressure(0.0);
                        }
                    }
                }
                self.release_held(channel);
            }
            Action::AllNotesOff => {
                for &index in &self.sounding {
                    let voice = &mut self.voices[index];
                    if voice.channel() == channel {
                        voice.note_off(1.0);
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

    /// Let go of any voice whose note-off arrived while a pedal was down and
    /// which no pedal is still holding.
    fn release_held(&mut self, channel: u8) {
        let sustain = self.channels[channel as usize].sustain;
        for &index in &self.sounding {
            let voice = &mut self.voices[index];
            if voice.channel() == channel
                && voice.is_sustained()
                && !sustain
                && !voice.is_sostenuto()
            {
                voice.release_held();
            }
        }
    }

    fn next_frame(&mut self) -> (f32, f32) {
        let mut dry_left = 0.0;
        let mut dry_right = 0.0;
        let mut send = 0.0;
        let mut position = 0;
        while position < self.sounding.len() {
            let index = self.sounding[position];
            let voice = &mut self.voices[index];
            if voice.is_active() {
                let state = &self.channels[voice.channel() as usize];
                let sample =
                    voice.next_sample(state.pitch_scale * self.master_tuning, state.vibrato());
                let level = sample * state.level();
                let (left, right) = state.pan;
                dry_left += level * left;
                dry_right += level * right;
                send += level * state.reverb_send;
                position += 1;
            } else {
                self.sounding.swap_remove(position);
                self.free.push(index);
            }
        }
        let gain = MASTER_GAIN * self.master_volume;
        let dry_left = dry_left * gain;
        let dry_right = dry_right * gain;
        let (wet_left, wet_right) = self.reverb.process(send * gain);
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
                if self.channels[channel as usize].mono {
                    for &index in &self.sounding {
                        let voice = &mut self.voices[index];
                        if voice.channel() == channel && voice.is_active() {
                            voice.note_off(1.0);
                        }
                    }
                }
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
            Command::NoteOff {
                channel,
                note,
                velocity,
            } => {
                let sustain = self.channels[channel as usize].sustain;
                let scale = release_scale(velocity);
                for &index in &self.sounding {
                    let voice = &mut self.voices[index];
                    if voice.is_active() && voice.matches(channel, note) {
                        if sustain || voice.is_sostenuto() {
                            voice.hold(scale);
                        } else {
                            voice.note_off(scale);
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
            Command::SetMasterTuning(semitones) => {
                self.master_tuning = 2.0f32.powf(semitones.clamp(-96.0, 96.0) / 12.0);
            }
            // Applied on the master bus, which sees every engine.
            Command::SetMasterBalance(_) => {}
            Command::Reset => {
                self.channels = std::array::from_fn(|_| Channel::default());
                self.master_volume = 1.0;
                self.master_tuning = 1.0;
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

/// How note-off velocity retimes the release. Velocity 0 means the file did not
/// specify one, which is how nearly every file is written, so it stays neutral.
fn release_scale(velocity: u8) -> f32 {
    if velocity == UNSPECIFIED_RELEASE_VELOCITY {
        return 1.0;
    }
    let offset = f32::from(NEUTRAL_RELEASE_VELOCITY) - f32::from(velocity);
    2.0f32.powf(offset / 64.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi::cc;

    impl Synth {
        fn held(&self, channel: u8) -> Vec<u8> {
            self.sounding
                .iter()
                .map(|&i| &self.voices[i])
                .filter(|v| v.channel() == channel && v.is_sustained())
                .filter_map(|v| v.note())
                .collect()
        }

        fn sounding_notes(&self, channel: u8) -> Vec<u8> {
            self.sounding
                .iter()
                .map(|&i| &self.voices[i])
                .filter(|v| v.channel() == channel && v.is_active())
                .filter_map(|v| v.note())
                .collect()
        }
    }

    fn note_on(synth: &mut Synth, note: u8) {
        synth.handle(Command::NoteOn {
            channel: 0,
            note,
            velocity: 100,
        });
    }

    fn note_off(synth: &mut Synth, note: u8) {
        synth.handle(Command::NoteOff {
            channel: 0,
            note,
            velocity: 64,
        });
    }

    fn control(synth: &mut Synth, controller: u8, value: u8) {
        synth.handle(Command::ControlChange {
            channel: 0,
            controller,
            value,
        });
    }

    fn synth() -> Synth {
        Synth::new(44_100.0)
    }

    /// Long enough for any release tail in these tests to reach silence.
    fn advance(synth: &mut Synth) {
        for _ in 0..44_100 * 3 {
            synth.next_frame();
        }
    }

    #[test]
    fn sostenuto_holds_only_what_was_already_down() {
        let mut synth = synth();
        note_on(&mut synth, 60);
        control(&mut synth, cc::SOSTENUTO, 127);
        note_on(&mut synth, 62);

        note_off(&mut synth, 60);
        note_off(&mut synth, 62);

        assert_eq!(synth.held(0), [60]);
    }

    #[test]
    fn sustain_holds_everything_until_it_lifts() {
        let mut synth = synth();
        control(&mut synth, cc::SUSTAIN, 127);
        note_on(&mut synth, 60);
        note_on(&mut synth, 62);
        note_off(&mut synth, 60);
        note_off(&mut synth, 62);
        assert_eq!(synth.held(0).len(), 2);

        control(&mut synth, cc::SUSTAIN, 0);
        advance(&mut synth);
        assert!(synth.sounding_notes(0).is_empty());
    }

    #[test]
    fn sostenuto_keeps_its_notes_when_sustain_lifts() {
        let mut synth = synth();
        note_on(&mut synth, 60);
        control(&mut synth, cc::SOSTENUTO, 127);
        control(&mut synth, cc::SUSTAIN, 127);
        note_off(&mut synth, 60);

        control(&mut synth, cc::SUSTAIN, 0);
        advance(&mut synth);
        assert_eq!(synth.sounding_notes(0), [60]);

        control(&mut synth, cc::SOSTENUTO, 0);
        advance(&mut synth);
        assert!(synth.sounding_notes(0).is_empty());
    }

    #[test]
    fn mono_mode_leaves_one_note_sounding() {
        let mut synth = synth();
        note_on(&mut synth, 60);
        note_on(&mut synth, 64);
        assert_eq!(synth.sounding_notes(0).len(), 2);

        control(&mut synth, cc::MONO_MODE, 1);
        note_on(&mut synth, 60);
        note_on(&mut synth, 64);
        advance(&mut synth);
        assert_eq!(synth.sounding_notes(0), [64]);
    }

    #[test]
    fn all_sound_off_frees_voices_where_all_notes_off_only_releases_them() {
        let mut synth = synth();
        note_on(&mut synth, 60);
        control(&mut synth, cc::ALL_NOTES_OFF, 0);
        assert_eq!(synth.sounding.len(), 1);

        note_on(&mut synth, 62);
        control(&mut synth, cc::ALL_SOUND_OFF, 0);
        assert!(synth.sounding.is_empty());
    }

    #[test]
    fn release_velocity_only_retimes_when_it_is_specified() {
        assert_eq!(release_scale(UNSPECIFIED_RELEASE_VELOCITY), 1.0);
        assert_eq!(release_scale(NEUTRAL_RELEASE_VELOCITY), 1.0);
        assert!(release_scale(127) < 1.0);
        assert!(release_scale(1) > 1.0);
    }
}
