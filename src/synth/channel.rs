use crate::midi::{
    CENTRE_14_BIT, DEFAULT_BEND_SEMITONES, DEFAULT_REVERB_SEND, PEDAL_THRESHOLD, RPN_COARSE_TUNING,
    RPN_FINE_TUNING, RPN_MODULATION_DEPTH_RANGE, RPN_NULL, RPN_PITCH_BEND_SENSITIVITY, cc,
    is_channel_mode, stereo_gains, volume_gain,
};

/// How much vibrato the modulation wheel adds per semitone of its range.
const MODULATION_PER_SEMITONE: f32 = 0.04;
/// The modulation range a channel starts with, in semitones.
const DEFAULT_MODULATION_SEMITONES: f32 = 0.5;
/// How far the soft pedal holds a note back.
const SOFT_PEDAL: f32 = 0.7;

/// The MIDI 1.0 state of one channel: everything a stream of controllers,
/// bends and pressure messages accumulates between notes.
pub struct Channel {
    pub program: u8,
    pub bank: u16,
    pub volume: f32,
    pub expression: f32,
    pub pan: (f32, f32),
    pub sustain: bool,
    pub sostenuto: bool,
    /// Mono mode holds one note at a time; poly mode is the General MIDI default.
    pub mono: bool,
    pub soft: f32,
    /// Mod wheel travel, 0.0..=1.0, and the depth a full sweep reaches.
    pub modulation: f32,
    pub modulation_range: f32,
    pub pressure: f32,
    pub reverb_send: f32,
    /// Sound controllers 71-74, each a multiplier centred on 1.0 at value 64.
    pub resonance: f32,
    pub release_scale: f32,
    pub attack_scale: f32,
    pub brightness_scale: f32,
    /// Bend in semitones, and the multiplier it works out to.
    pub bend_semitones: f32,
    pub bend_range: f32,
    /// Channel tuning from RPNs 1 and 2, in semitones, kept apart so that one
    /// cannot round the other away.
    pub coarse_tuning: f32,
    pub fine_tuning: f32,
    pub pitch_scale: f32,
    /// The parameter number data entry is currently pointed at. `None` once a
    /// non-registered parameter is selected, which we do not implement.
    parameter: Option<(u8, u8)>,
    /// The most recent data entry value, as (MSB, LSB).
    data: (u8, u8),
}

/// What the synth must do in response, beyond updating channel state.
pub enum Action {
    None,
    /// Release every note on the channel, honouring the pedals.
    AllNotesOff,
    /// Cut every note on the channel dead, ignoring the pedals.
    AllSoundOff,
    /// The sustain pedal came up.
    SustainReleased,
    /// The sostenuto pedal went down: capture whatever is sounding now.
    SostenutoPressed,
    /// The sostenuto pedal came up.
    SostenutoReleased,
    /// Reset all controllers: pedals up, per-note pressure gone.
    ResetControllers,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            program: 0,
            bank: 0,
            volume: volume_gain(100),
            expression: 1.0,
            pan: pan_gains(64),
            sustain: false,
            sostenuto: false,
            mono: false,
            soft: 1.0,
            modulation: 0.0,
            modulation_range: DEFAULT_MODULATION_SEMITONES,
            pressure: 0.0,
            reverb_send: DEFAULT_REVERB_SEND,
            resonance: 1.0,
            release_scale: 1.0,
            attack_scale: 1.0,
            brightness_scale: 1.0,
            bend_semitones: 0.0,
            bend_range: DEFAULT_BEND_SEMITONES,
            coarse_tuning: 0.0,
            fine_tuning: 0.0,
            pitch_scale: 1.0,
            parameter: None,
            data: (0, 0),
        }
    }
}

impl Channel {
    /// Apply a controller, returning whatever the voices still need doing.
    pub fn control(&mut self, controller: u8, value: u8) -> Action {
        if is_channel_mode(controller) {
            return self.channel_mode(controller);
        }
        let unit = f32::from(value) / 127.0;
        match controller {
            cc::BANK_MSB => self.bank = (self.bank & 0x7f) | (u16::from(value) << 7),
            cc::BANK_LSB => self.bank = (self.bank & !0x7f) | u16::from(value),
            cc::VOLUME => self.volume = volume_gain(value),
            cc::EXPRESSION => self.expression = volume_gain(value),
            cc::PAN => self.pan = pan_gains(value),
            cc::MODULATION => self.modulation = unit,
            cc::REVERB_SEND => self.reverb_send = unit,
            cc::SOFT => self.soft = 1.0 - unit * (1.0 - SOFT_PEDAL),
            // Sound controllers are relative, with 64 meaning "as the patch is".
            cc::RESONANCE => self.resonance = centred(value, 2.0),
            cc::RELEASE_TIME => self.release_scale = centred(value, 4.0),
            cc::ATTACK_TIME => self.attack_scale = centred(value, 4.0),
            cc::BRIGHTNESS => self.brightness_scale = centred(value, 4.0),
            cc::SUSTAIN => {
                let down = value >= PEDAL_THRESHOLD;
                let released = self.sustain && !down;
                self.sustain = down;
                if released {
                    return Action::SustainReleased;
                }
            }
            cc::SOSTENUTO => {
                let down = value >= PEDAL_THRESHOLD;
                let changed = self.sostenuto != down;
                self.sostenuto = down;
                if changed {
                    return if down {
                        Action::SostenutoPressed
                    } else {
                        Action::SostenutoReleased
                    };
                }
            }
            cc::RPN_MSB => self.select_parameter(Some(value), None),
            cc::RPN_LSB => self.select_parameter(None, Some(value)),
            // Selecting a non-registered parameter takes data entry out of play.
            cc::NRPN_MSB | cc::NRPN_LSB => self.parameter = None,
            // An MSB write restarts the value, so a file that sends only the
            // coarse half still lands on the right number.
            cc::DATA_ENTRY_MSB => {
                self.data = (value, 0);
                self.apply_parameter();
            }
            cc::DATA_ENTRY_LSB => {
                self.data.1 = value;
                self.apply_parameter();
            }
            cc::DATA_INCREMENT => self.nudge_parameter(1),
            cc::DATA_DECREMENT => self.nudge_parameter(-1),
            _ => {}
        }
        Action::None
    }

    fn select_parameter(&mut self, msb: Option<u8>, lsb: Option<u8>) {
        let (held_msb, held_lsb) = self.parameter.unwrap_or(RPN_NULL);
        let selected = (msb.unwrap_or(held_msb), lsb.unwrap_or(held_lsb));
        self.parameter = (selected != RPN_NULL).then_some(selected);
    }

    /// Bend sensitivity steps by a whole semitone; the rest step by their LSB.
    fn nudge_parameter(&mut self, delta: i32) {
        let step = match self.parameter {
            Some(RPN_PITCH_BEND_SENSITIVITY) => 128,
            _ => 1,
        };
        let raw = (i32::from(self.data.0) << 7 | i32::from(self.data.1)) + delta * step;
        let raw = raw.clamp(0, 16383);
        self.data = ((raw >> 7) as u8, (raw & 0x7f) as u8);
        self.apply_parameter();
    }

    fn apply_parameter(&mut self) {
        let (msb, lsb) = self.data;
        let wide = i32::from(msb) << 7 | i32::from(lsb);
        match self.parameter {
            Some(RPN_PITCH_BEND_SENSITIVITY) => {
                // MSB is semitones, LSB cents.
                self.bend_range = f32::from(msb) + f32::from(lsb) / 100.0;
                self.refresh_pitch();
            }
            Some(RPN_FINE_TUNING) => {
                self.fine_tuning = (wide - CENTRE_14_BIT) as f32 / 8192.0;
                self.refresh_pitch();
            }
            Some(RPN_COARSE_TUNING) => {
                self.coarse_tuning = f32::from(msb) - 64.0;
                self.refresh_pitch();
            }
            Some(RPN_MODULATION_DEPTH_RANGE) => {
                self.modulation_range = f32::from(msb) + f32::from(lsb) / 128.0;
            }
            _ => {}
        }
    }

    fn channel_mode(&mut self, controller: u8) -> Action {
        match controller {
            cc::ALL_SOUND_OFF => Action::AllSoundOff,
            cc::RESET_ALL_CONTROLLERS => {
                // Volume, pan and program deliberately survive a reset.
                self.expression = 1.0;
                self.modulation = 0.0;
                self.pressure = 0.0;
                self.sustain = false;
                self.sostenuto = false;
                self.soft = 1.0;
                self.reverb_send = DEFAULT_REVERB_SEND;
                self.modulation_range = DEFAULT_MODULATION_SEMITONES;
                self.resonance = 1.0;
                self.release_scale = 1.0;
                self.attack_scale = 1.0;
                self.brightness_scale = 1.0;
                self.bend_semitones = 0.0;
                self.pitch_scale = 1.0;
                self.parameter = None;
                self.data = (0, 0);
                Action::ResetControllers
            }
            // Local control has no meaning without a local keyboard.
            cc::LOCAL_CONTROL => Action::None,
            // Mode changes imply all notes off. Omni is meaningless to a player
            // reading a file, but mono is not: it collapses the channel.
            cc::MONO_MODE => {
                self.mono = true;
                Action::AllNotesOff
            }
            cc::POLY_MODE => {
                self.mono = false;
                Action::AllNotesOff
            }
            cc::ALL_NOTES_OFF | cc::OMNI_OFF | cc::OMNI_ON => Action::AllNotesOff,
            _ => Action::None,
        }
    }

    /// `offset` is the raw 14-bit distance from centre.
    pub fn set_bend(&mut self, offset: i16) {
        self.bend_semitones = f32::from(offset) / 8192.0 * self.bend_range;
        self.refresh_pitch();
    }

    fn refresh_pitch(&mut self) {
        let semitones =
            (self.bend_semitones + self.coarse_tuning + self.fine_tuning).clamp(-96.0, 96.0);
        self.pitch_scale = 2.0f32.powf(semitones / 12.0);
    }

    /// Total vibrato the channel is asking for, from the wheel and aftertouch.
    pub fn vibrato(&self) -> f32 {
        let depth = self.modulation_range * MODULATION_PER_SEMITONE;
        (self.modulation + self.pressure) * depth
    }

    /// Gain a note struck now should be played at.
    pub fn level(&self) -> f32 {
        self.volume * self.expression
    }
}

pub fn pan_gains(position: u8) -> (f32, f32) {
    stereo_gains(f32::from(position) / 63.5 - 1.0)
}

/// A sound controller reading, as a multiplier: 0 gives `1/range`, 64 gives 1.0
/// and 127 gives `range`.
fn centred(value: u8, range: f32) -> f32 {
    let offset = (f32::from(value) - 64.0) / 63.0;
    range.powf(offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rpn(channel: &mut Channel, msb: u8, lsb: u8) {
        channel.control(cc::RPN_MSB, msb);
        channel.control(cc::RPN_LSB, lsb);
    }

    fn semitones(channel: &Channel) -> f32 {
        channel.pitch_scale.log2() * 12.0
    }

    #[test]
    fn bend_sensitivity_widens_the_bend() {
        let mut channel = Channel::default();
        channel.set_bend(8191);
        assert!((semitones(&channel) - 2.0).abs() < 0.01);

        rpn(&mut channel, 0, 0);
        channel.control(cc::DATA_ENTRY_MSB, 12);
        channel.set_bend(8191);
        assert!((semitones(&channel) - 12.0).abs() < 0.01);
    }

    #[test]
    fn coarse_and_fine_tuning_do_not_clobber_each_other() {
        let mut channel = Channel::default();

        rpn(&mut channel, 0, 2);
        channel.control(cc::DATA_ENTRY_MSB, 62);
        assert!((semitones(&channel) + 2.0).abs() < 1e-4);

        rpn(&mut channel, 0, 1);
        channel.control(cc::DATA_ENTRY_MSB, 32);
        channel.control(cc::DATA_ENTRY_LSB, 0);
        assert!((semitones(&channel) + 2.5).abs() < 1e-4);

        // Re-sending the coarse value must leave the fine offset intact.
        rpn(&mut channel, 0, 2);
        channel.control(cc::DATA_ENTRY_MSB, 62);
        assert!((semitones(&channel) + 2.5).abs() < 1e-4);
    }

    #[test]
    fn the_null_parameter_parks_data_entry() {
        let mut channel = Channel::default();
        rpn(&mut channel, 127, 127);
        channel.control(cc::DATA_ENTRY_MSB, 12);
        assert_eq!(channel.bend_range, DEFAULT_BEND_SEMITONES);
    }

    #[test]
    fn a_non_registered_parameter_parks_data_entry() {
        let mut channel = Channel::default();
        rpn(&mut channel, 0, 0);
        channel.control(cc::NRPN_MSB, 1);
        channel.control(cc::DATA_ENTRY_MSB, 12);
        assert_eq!(channel.bend_range, DEFAULT_BEND_SEMITONES);
    }

    #[test]
    fn data_increment_steps_bend_sensitivity_by_a_semitone() {
        let mut channel = Channel::default();
        rpn(&mut channel, 0, 0);
        channel.control(cc::DATA_ENTRY_MSB, 2);
        channel.control(cc::DATA_INCREMENT, 0);
        assert_eq!(channel.bend_range, 3.0);
        channel.control(cc::DATA_DECREMENT, 0);
        channel.control(cc::DATA_DECREMENT, 0);
        assert_eq!(channel.bend_range, 1.0);
    }

    #[test]
    fn the_pedals_report_only_their_edges() {
        let mut channel = Channel::default();
        assert!(matches!(channel.control(cc::SUSTAIN, 127), Action::None));
        assert!(matches!(channel.control(cc::SUSTAIN, 127), Action::None));
        assert!(matches!(
            channel.control(cc::SUSTAIN, 0),
            Action::SustainReleased
        ));

        assert!(matches!(
            channel.control(cc::SOSTENUTO, 127),
            Action::SostenutoPressed
        ));
        assert!(matches!(channel.control(cc::SOSTENUTO, 100), Action::None));
        assert!(matches!(
            channel.control(cc::SOSTENUTO, 63),
            Action::SostenutoReleased
        ));
    }

    #[test]
    fn mono_and_poly_mode_switch_the_channel() {
        let mut channel = Channel::default();
        assert!(!channel.mono);
        assert!(matches!(
            channel.control(cc::MONO_MODE, 1),
            Action::AllNotesOff
        ));
        assert!(channel.mono);
        assert!(matches!(
            channel.control(cc::POLY_MODE, 0),
            Action::AllNotesOff
        ));
        assert!(!channel.mono);
    }

    #[test]
    fn reset_all_controllers_spares_volume_pan_and_program() {
        let mut channel = Channel {
            program: 40,
            ..Default::default()
        };
        channel.control(cc::VOLUME, 20);
        channel.control(cc::PAN, 0);
        channel.control(cc::EXPRESSION, 10);
        channel.control(cc::SUSTAIN, 127);
        channel.set_bend(4000);

        let volume = channel.volume;
        let pan = channel.pan;
        assert!(matches!(
            channel.control(cc::RESET_ALL_CONTROLLERS, 0),
            Action::ResetControllers
        ));

        assert_eq!(channel.program, 40);
        assert_eq!(channel.volume, volume);
        assert_eq!(channel.pan, pan);
        assert_eq!(channel.expression, 1.0);
        assert!(!channel.sustain);
        assert_eq!(channel.pitch_scale, 1.0);
    }

    #[test]
    fn volume_and_expression_multiply_into_the_level() {
        let mut channel = Channel::default();
        channel.control(cc::VOLUME, 127);
        channel.control(cc::EXPRESSION, 127);
        assert_eq!(channel.level(), 1.0);
        channel.control(cc::EXPRESSION, 64);
        assert!((channel.level() - 0.254).abs() < 0.001);
    }
}
