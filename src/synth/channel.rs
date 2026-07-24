use crate::midi::{
    DEFAULT_BEND_SEMITONES, DEFAULT_REVERB_SEND, PEDAL_THRESHOLD, RPN_COARSE_TUNING,
    RPN_FINE_TUNING, RPN_MODULATION_DEPTH_RANGE, RPN_PITCH_BEND_SENSITIVITY, cc, is_channel_mode,
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
    /// Channel tuning from RPNs 1 and 2, in semitones.
    pub tuning: f32,
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
    /// The sustain or sostenuto pedal came up.
    PedalReleased,
    /// Vibrato depth changed and sounding notes should follow.
    ModulationChanged,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            program: 0,
            bank: 0,
            volume: 100.0 / 127.0,
            expression: 1.0,
            pan: pan_gains(64),
            sustain: false,
            sostenuto: false,
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
            tuning: 0.0,
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
            cc::VOLUME => self.volume = unit,
            cc::EXPRESSION => self.expression = unit,
            cc::PAN => self.pan = pan_gains(value),
            cc::MODULATION => {
                self.modulation = unit;
                return Action::ModulationChanged;
            }
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
                    return Action::PedalReleased;
                }
            }
            cc::SOSTENUTO => {
                let down = value >= PEDAL_THRESHOLD;
                let released = self.sostenuto && !down;
                self.sostenuto = down;
                if released {
                    return Action::PedalReleased;
                }
            }
            cc::RPN_MSB => self.parameter = Some((value, self.parameter.map_or(0, |(_, l)| l))),
            cc::RPN_LSB => self.parameter = Some((self.parameter.map_or(0, |(m, _)| m), value)),
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
            cc::DATA_INCREMENT => {
                self.data.0 = self.data.0.saturating_add(1).min(127);
                self.apply_parameter();
            }
            cc::DATA_DECREMENT => {
                self.data.0 = self.data.0.saturating_sub(1);
                self.apply_parameter();
            }
            _ => {}
        }
        Action::None
    }

    fn apply_parameter(&mut self) {
        let (msb, lsb) = self.data;
        match self.parameter {
            Some(RPN_PITCH_BEND_SENSITIVITY) => {
                // MSB is semitones, LSB cents.
                self.bend_range = f32::from(msb) + f32::from(lsb) / 100.0;
                self.refresh_pitch();
            }
            Some(RPN_FINE_TUNING) => {
                // 14-bit, centred on 8192, spanning a semitone either way.
                let raw = i32::from(msb) << 7 | i32::from(lsb);
                self.tuning = self.tuning.trunc() + (raw - 8192) as f32 / 8192.0;
                self.refresh_pitch();
            }
            Some(RPN_COARSE_TUNING) => {
                self.tuning = f32::from(msb) - 64.0 + self.tuning.fract();
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
                Action::PedalReleased
            }
            // Local control has no meaning without a local keyboard.
            cc::LOCAL_CONTROL => Action::None,
            // Mode changes imply all notes off; we are always omni-off and poly.
            cc::ALL_NOTES_OFF | cc::OMNI_OFF | cc::OMNI_ON | cc::MONO_MODE | cc::POLY_MODE => {
                Action::AllNotesOff
            }
            _ => Action::None,
        }
    }

    /// `offset` is the raw 14-bit distance from centre.
    pub fn set_bend(&mut self, offset: i16) {
        self.bend_semitones = f32::from(offset) / 8192.0 * self.bend_range;
        self.refresh_pitch();
    }

    fn refresh_pitch(&mut self) {
        let semitones = (self.bend_semitones + self.tuning).clamp(-96.0, 96.0);
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

/// Equal-power pan: a centred channel keeps unity gain on both sides, and the
/// total power stays constant as it moves across the image.
pub fn pan_gains(position: u8) -> (f32, f32) {
    let angle = f32::from(position) / 127.0 * std::f32::consts::FRAC_PI_2;
    (
        angle.cos() * std::f32::consts::SQRT_2,
        angle.sin() * std::f32::consts::SQRT_2,
    )
}

/// A sound controller reading, as a multiplier: 0 gives `1/range`, 64 gives 1.0
/// and 127 gives `range`.
fn centred(value: u8, range: f32) -> f32 {
    let offset = (f32::from(value) - 64.0) / 63.0;
    range.powf(offset)
}
