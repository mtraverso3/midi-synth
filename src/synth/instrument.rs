use super::oscillator::Waveform;

/// MIDI channel 9 (0-indexed) is reserved for percussion by the General MIDI
/// standard, regardless of program.
const DRUM_CHANNEL: u8 = 9;

/// Pick a waveform for a channel from its General MIDI program number, grouping
/// programs into instrument families that share a similar timbre.
pub fn waveform_for(channel: u8, program: u8) -> Waveform {
    if channel == DRUM_CHANNEL {
        return Waveform::Square;
    }
    match program {
        0..=15 => Waveform::Triangle,  // pianos, chromatic percussion
        16..=23 => Waveform::Sine,     // organs
        24..=39 => Waveform::Triangle, // guitars, basses
        40..=63 => Waveform::Saw,      // strings, ensembles, brass
        64..=71 => Waveform::Square,   // reeds
        72..=79 => Waveform::Sine,     // pipes, flutes
        _ => Waveform::Square,         // synth leads and everything else
    }
}
