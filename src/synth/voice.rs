use super::envelope::Envelope;
use super::instrument::Instrument;
use super::oscillator::{Oscillator, Waveform};

pub struct Voice {
    oscillator: Oscillator,
    envelope: Envelope,
    channel: u8,
    note: Option<u8>,
    amplitude: f32,
}

impl Voice {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            oscillator: Oscillator::new(sample_rate, Waveform::Triangle),
            envelope: Envelope::new(sample_rate),
            channel: 0,
            note: None,
            amplitude: 0.0,
        }
    }

    pub fn is_active(&self) -> bool {
        !self.envelope.is_finished()
    }

    pub fn matches(&self, channel: u8, note: u8) -> bool {
        self.note == Some(note) && self.channel == channel
    }

    pub fn note_on(&mut self, channel: u8, note: u8, velocity: u8, instrument: Instrument) {
        self.channel = channel;
        self.note = Some(note);
        self.amplitude = velocity as f32 / 127.0;
        self.oscillator.set_waveform(instrument.waveform);
        self.oscillator.set_frequency(note_to_freq(note));
        self.envelope.configure(
            instrument.attack_s,
            instrument.decay_s,
            instrument.sustain_level,
            instrument.release_s,
        );
        self.envelope.trigger();
    }

    pub fn note_off(&mut self) {
        self.envelope.release();
    }

    pub fn next_sample(&mut self) -> f32 {
        self.oscillator.next_sample() * self.envelope.next_sample() * self.amplitude
    }
}

fn note_to_freq(note: u8) -> f32 {
    440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0)
}
