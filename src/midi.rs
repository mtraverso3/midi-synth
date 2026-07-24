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
    pub const RESONANCE: u8 = 71;
    pub const RELEASE_TIME: u8 = 72;
    pub const ATTACK_TIME: u8 = 73;
    pub const BRIGHTNESS: u8 = 74;
    pub const SOSTENUTO: u8 = 66;
    pub const REVERB_SEND: u8 = 91;
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

/// The registered parameters this synth understands.
pub const RPN_PITCH_BEND_SENSITIVITY: (u8, u8) = (0, 0);
pub const RPN_FINE_TUNING: (u8, u8) = (0, 1);
pub const RPN_COARSE_TUNING: (u8, u8) = (0, 2);
pub const RPN_MODULATION_DEPTH_RANGE: (u8, u8) = (0, 5);

/// General MIDI starts every channel with this much reverb send.
pub const DEFAULT_REVERB_SEND: f32 = 40.0 / 127.0;

/// A note-off carrying velocity 0 means "unspecified" rather than "as slow as
/// possible", which is how the overwhelming majority of files are written.
pub const UNSPECIFIED_RELEASE_VELOCITY: u8 = 0;
pub const NEUTRAL_RELEASE_VELOCITY: u8 = 64;

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

/// The universal system exclusive messages worth acting on. Everything else is
/// device-specific and safely ignored.
pub enum SystemExclusive {
    /// Universal master volume, 14-bit.
    MasterVolume(u16),
    /// "GM System On": reset every channel to General MIDI defaults.
    GeneralMidiReset,
}

/// Recognise a SysEx payload. `data` excludes the leading 0xF0 but may include
/// the trailing 0xF7, as midly reports it.
pub fn parse_system_exclusive(data: &[u8]) -> Option<SystemExclusive> {
    let data = data.strip_suffix(&[0xF7]).unwrap_or(data);
    match data {
        // F0 7F <device> 04 01 <lsb> <msb> F7
        [0x7F, _, 0x04, 0x01, lsb, msb] => Some(SystemExclusive::MasterVolume(
            u16::from(*msb) << 7 | u16::from(*lsb),
        )),
        // F0 7E <device> 09 01 F7
        [0x7E, _, 0x09, 0x01] => Some(SystemExclusive::GeneralMidiReset),
        _ => None,
    }
}
