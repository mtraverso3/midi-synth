//! Shared MIDI 1.0 vocabulary: the controller numbers and conventions the
//! parser, sequencer and synths all have to agree on.

/// Controller numbers this player acts on. Others are carried through to the
/// engines regardless, so a SoundFont can honour ones we don't interpret.
pub mod cc {
    pub const BANK_MSB: u8 = 0;
    pub const MODULATION: u8 = 1;
    pub const DATA_ENTRY_MSB: u8 = 6;
    pub const VOLUME: u8 = 7;
    pub const PAN: u8 = 10;
    pub const EXPRESSION: u8 = 11;
    pub const BANK_LSB: u8 = 32;
    pub const DATA_ENTRY_LSB: u8 = 38;
    pub const SUSTAIN: u8 = 64;
    pub const SOSTENUTO: u8 = 66;
    pub const SOFT: u8 = 67;
    pub const DATA_INCREMENT: u8 = 96;
    pub const DATA_DECREMENT: u8 = 97;
    pub const NRPN_LSB: u8 = 98;
    pub const NRPN_MSB: u8 = 99;
    pub const RPN_LSB: u8 = 100;
    pub const RPN_MSB: u8 = 101;

    // Channel mode messages occupy 120..=127.
    pub const ALL_SOUND_OFF: u8 = 120;
    pub const RESET_ALL_CONTROLLERS: u8 = 121;
    pub const LOCAL_CONTROL: u8 = 122;
    pub const ALL_NOTES_OFF: u8 = 123;
    pub const OMNI_OFF: u8 = 124;
    pub const OMNI_ON: u8 = 125;
    pub const MONO_MODE: u8 = 126;
    pub const POLY_MODE: u8 = 127;
}

/// Registered parameter 0: how far a full pitch bend reaches.
pub const RPN_PITCH_BEND_SENSITIVITY: (u8, u8) = (0, 0);

/// Pitch bend is 14-bit, centred at 8192.
pub const PITCH_BEND_CENTRE: i16 = 8192;

/// A controller value of 64 or more counts as a pedal being down.
pub const PEDAL_THRESHOLD: u8 = 64;

/// The default bend range when a file never sets RPN 0, per General MIDI.
pub const DEFAULT_BEND_SEMITONES: f32 = 2.0;

/// Channel mode messages all silence or reset the channel rather than setting
/// a continuous value.
pub fn is_channel_mode(controller: u8) -> bool {
    controller >= cc::ALL_SOUND_OFF
}

/// Whether a controller carries state that must be replayed after a seek.
/// One-shot messages (the channel mode group) must not be.
pub fn is_persistent(controller: u8) -> bool {
    !is_channel_mode(controller)
}
