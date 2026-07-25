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

/// Selecting RPN 127/127 deliberately parks data entry so that stray data
/// messages cannot disturb whichever parameter was last addressed.
pub const RPN_NULL: (u8, u8) = (127, 127);

/// A 14-bit value centred on 8192, as the tuning RPNs and pitch bend use.
pub const CENTRE_14_BIT: i32 = 8192;

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

/// The gain a volume-like controller asks for. General MIDI 2 and DLS both
/// define channel volume and expression as an attenuation of 40 dB across the
/// range, which is the square of the fraction rather than the fraction itself.
/// Read linearly, a fade to CC 7 = 32 only drops 12 dB where it should drop 24,
/// so mixes come out flat and quiet passages far too loud.
pub fn volume_gain(value: u8) -> f32 {
    let unit = f32::from(value) / 127.0;
    unit * unit
}

/// Equal-power stereo placement for `position` in -1.0..=1.0: centred keeps
/// unity gain on both sides, and total power stays constant across the image.
pub fn stereo_gains(position: f32) -> (f32, f32) {
    let angle = (position.clamp(-1.0, 1.0) + 1.0) / 2.0 * std::f32::consts::FRAC_PI_2;
    (
        angle.cos() * std::f32::consts::SQRT_2,
        angle.sin() * std::f32::consts::SQRT_2,
    )
}

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
    /// Universal master balance, 14-bit and centred on 8192.
    MasterBalance(u16),
    /// Universal master fine tuning, 14-bit and centred on 8192, spanning a
    /// semitone either way.
    MasterFineTuning(u16),
    /// Universal master coarse tuning; only the MSB is meaningful, in semitones
    /// offset from 64.
    MasterCoarseTuning(u8),
    /// "GM System On" (or its GM2 and GM-off siblings): reset every channel to
    /// General MIDI defaults.
    GeneralMidiReset,
}

/// Recognise a SysEx payload. `data` excludes the leading 0xF0 but may include
/// the trailing 0xF7, as midly reports it.
pub fn parse_system_exclusive(data: &[u8]) -> Option<SystemExclusive> {
    let data = data.strip_suffix(&[0xF7]).unwrap_or(data);
    let wide = |lsb: &u8, msb: &u8| u16::from(*msb) << 7 | u16::from(*lsb);
    match data {
        // Universal real time, F0 7F <device> 04 <sub> <lsb> <msb> F7.
        [0x7F, _, 0x04, 0x01, lsb, msb] => Some(SystemExclusive::MasterVolume(wide(lsb, msb))),
        [0x7F, _, 0x04, 0x02, lsb, msb] => Some(SystemExclusive::MasterBalance(wide(lsb, msb))),
        [0x7F, _, 0x04, 0x03, lsb, msb] => Some(SystemExclusive::MasterFineTuning(wide(lsb, msb))),
        [0x7F, _, 0x04, 0x04, _, msb] => Some(SystemExclusive::MasterCoarseTuning(*msb)),
        // Universal non-real time, F0 7E <device> 09 <mode> F7. All three modes
        // — GM on, GM off and GM2 on — put the instrument back to its defaults;
        // we have only the one sound set to offer either way.
        [0x7E, _, 0x09, 0x01..=0x03] => Some(SystemExclusive::GeneralMidiReset),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sysex(data: &[u8]) -> Option<SystemExclusive> {
        parse_system_exclusive(data)
    }

    #[test]
    fn reads_master_volume_either_side_of_the_terminator() {
        let expected = 8192;
        for payload in [
            &[0x7F, 0x7F, 0x04, 0x01, 0x00, 0x40][..],
            &[0x7F, 0x7F, 0x04, 0x01, 0x00, 0x40, 0xF7][..],
        ] {
            match sysex(payload) {
                Some(SystemExclusive::MasterVolume(level)) => assert_eq!(level, expected),
                _ => panic!("not recognised: {payload:02X?}"),
            }
        }
    }

    #[test]
    fn reads_the_universal_tuning_and_balance_messages() {
        assert!(matches!(
            sysex(&[0x7F, 0x00, 0x04, 0x02, 0x00, 0x40, 0xF7]),
            Some(SystemExclusive::MasterBalance(8192))
        ));
        assert!(matches!(
            sysex(&[0x7F, 0x00, 0x04, 0x03, 0x00, 0x50, 0xF7]),
            Some(SystemExclusive::MasterFineTuning(10240))
        ));
        assert!(matches!(
            sysex(&[0x7F, 0x00, 0x04, 0x04, 0x00, 0x42, 0xF7]),
            Some(SystemExclusive::MasterCoarseTuning(0x42))
        ));
    }

    #[test]
    fn every_general_midi_mode_message_resets() {
        for mode in [0x01, 0x02, 0x03] {
            assert!(matches!(
                sysex(&[0x7E, 0x7F, 0x09, mode, 0xF7]),
                Some(SystemExclusive::GeneralMidiReset)
            ));
        }
    }

    #[test]
    fn ignores_manufacturer_sysex() {
        assert!(sysex(&[0x41, 0x10, 0x42, 0x12, 0xF7]).is_none());
        assert!(sysex(&[]).is_none());
    }

    #[test]
    fn volume_is_a_square_law_taper() {
        assert_eq!(volume_gain(127), 1.0);
        assert_eq!(volume_gain(0), 0.0);
        // Half travel should be a quarter of the power, roughly -12 dB.
        assert!((volume_gain(64) - 0.254).abs() < 0.001);
    }

    #[test]
    fn stereo_placement_holds_its_power() {
        let power = |(l, r): (f32, f32)| l * l + r * r;
        for position in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            assert!((power(stereo_gains(position)) - 2.0).abs() < 1e-5);
        }
        assert_eq!(stereo_gains(0.0).0, stereo_gains(0.0).1);
        assert!(stereo_gains(-1.0).1.abs() < 1e-6);
        assert!(stereo_gains(2.0).0.abs() < 1e-6);
    }

    #[test]
    fn only_the_top_controllers_are_channel_modes() {
        assert!(!is_channel_mode(cc::SUSTAIN));
        assert!(is_channel_mode(cc::ALL_SOUND_OFF));
        assert!(is_channel_mode(cc::POLY_MODE));
        assert!(is_persistent(cc::VOLUME));
        assert!(!is_persistent(cc::ALL_NOTES_OFF));
    }
}
