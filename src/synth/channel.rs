use crate::midi::{
    DEFAULT_BEND_SEMITONES, PEDAL_THRESHOLD, RPN_PITCH_BEND_SENSITIVITY, cc, is_channel_mode,
};

/// How much vibrato the modulation wheel adds at full travel.
const MODULATION_DEPTH: f32 = 0.02;
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
    pub modulation: f32,
    pub pressure: f32,
    /// Bend in semitones, and the multiplier it works out to.
    pub bend_semitones: f32,
    pub bend_range: f32,
    pub pitch_scale: f32,
    /// The parameter number data entry is currently pointed at. `None` once a
    /// non-registered parameter is selected, which we do not implement.
    parameter: Option<(u8, u8)>,
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
            pressure: 0.0,
            bend_semitones: 0.0,
            bend_range: DEFAULT_BEND_SEMITONES,
            pitch_scale: 1.0,
            parameter: None,
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
                self.modulation = unit * MODULATION_DEPTH;
                return Action::ModulationChanged;
            }
            cc::SOFT => self.soft = 1.0 - unit * (1.0 - SOFT_PEDAL),
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
            cc::DATA_ENTRY_MSB => self.data_entry(f32::from(value)),
            cc::DATA_ENTRY_LSB => self.data_entry_fine(f32::from(value)),
            cc::DATA_INCREMENT => self.data_entry(self.bend_range + 1.0),
            cc::DATA_DECREMENT => self.data_entry(self.bend_range - 1.0),
            _ => {}
        }
        Action::None
    }

    fn data_entry(&mut self, semitones: f32) {
        if self.parameter == Some(RPN_PITCH_BEND_SENSITIVITY) {
            self.bend_range = semitones.clamp(0.0, 96.0);
            self.refresh_bend();
        }
    }

    fn data_entry_fine(&mut self, cents: f32) {
        if self.parameter == Some(RPN_PITCH_BEND_SENSITIVITY) {
            self.bend_range = self.bend_range.trunc() + cents / 100.0;
            self.refresh_bend();
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
                self.bend_semitones = 0.0;
                self.pitch_scale = 1.0;
                self.parameter = None;
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
        self.refresh_bend();
    }

    fn refresh_bend(&mut self) {
        self.bend_semitones = self.bend_semitones.clamp(-96.0, 96.0);
        self.pitch_scale = 2.0f32.powf(self.bend_semitones / 12.0);
    }

    /// Total vibrato the channel is asking for, from the wheel and aftertouch.
    pub fn vibrato(&self) -> f32 {
        self.modulation + self.pressure * MODULATION_DEPTH
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
