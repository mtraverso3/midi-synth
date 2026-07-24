use super::envelope::Envelope;
use super::filter::LowPass;
use super::instrument::Instrument;
use super::oscillator::{Oscillator, Waveform};

/// Second-oscillator detune (~10 cents); the drift against the first thickens the tone.
const DETUNE: f32 = 1.006;

pub struct Voice {
    sample_rate: f32,
    oscillator: Oscillator,
    detuned: Oscillator,
    envelope: Envelope,
    filter: LowPass,
    channel: u8,
    note: Option<u8>,
    amplitude: f32,
    /// Note-off arrived but the sustain pedal is holding this voice.
    sustained: bool,
}

impl Voice {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            oscillator: Oscillator::new(sample_rate, Waveform::Triangle),
            detuned: Oscillator::new(sample_rate, Waveform::Triangle),
            envelope: Envelope::new(sample_rate),
            filter: LowPass::new(),
            channel: 0,
            note: None,
            amplitude: 0.0,
            sustained: false,
        }
    }

    pub fn is_active(&self) -> bool {
        !self.envelope.is_finished()
    }

    pub fn channel(&self) -> u8 {
        self.channel
    }

    pub fn is_sustained(&self) -> bool {
        self.sustained
    }

    pub fn hold(&mut self) {
        self.sustained = true;
    }

    /// Current loudness (0.0..1.0), used to steal the quietest voice.
    pub fn level(&self) -> f32 {
        self.envelope.level() * self.amplitude
    }

    pub fn matches(&self, channel: u8, note: u8) -> bool {
        self.note == Some(note) && self.channel == channel
    }

    pub fn note_on(&mut self, channel: u8, note: u8, velocity: u8, instrument: Instrument) {
        self.channel = channel;
        self.note = Some(note);
        self.amplitude = velocity as f32 / 127.0;
        self.sustained = false;

        let freq = note_to_freq(note);
        self.oscillator.set_waveform(instrument.waveform);
        self.oscillator.set_frequency(freq);
        self.detuned.set_waveform(instrument.waveform);
        self.detuned.set_frequency(freq * DETUNE);
        self.filter
            .set_cutoff(self.sample_rate, instrument.cutoff_hz);

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

    pub fn kill(&mut self) {
        self.envelope.reset();
        self.note = None;
        self.sustained = false;
    }

    pub fn next_sample(&mut self) -> f32 {
        let osc = (self.oscillator.next_sample() + self.detuned.next_sample()) * 0.5;
        let filtered = self.filter.process(osc);
        filtered * self.envelope.next_sample() * self.amplitude
    }
}

fn note_to_freq(note: u8) -> f32 {
    440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0)
}
