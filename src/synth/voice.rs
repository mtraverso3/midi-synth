use super::envelope::Envelope;
use super::filter::LowPass;
use super::instrument::Instrument;
use super::oscillator::{Oscillator, Waveform};
use super::rng::Rng;

/// Second-oscillator detune (~5 cents), mixed in under the first: matched levels
/// would beat against each other and periodically cancel the note outright.
const DETUNE: f32 = 1.003;
const DETUNE_MIX: f32 = 0.4;
/// Per-note jitter of detune and level, so repeated notes aren't identical.
const DETUNE_JITTER: f32 = 0.0015;
const LEVEL_JITTER: f32 = 0.08;

const VIBRATO_HZ: f32 = 5.2;
/// Players ease vibrato in rather than starting a note with it.
const VIBRATO_ONSET_S: f32 = 0.35;

pub struct Voice {
    sample_rate: f32,
    oscillator: Oscillator,
    detuned: Oscillator,
    noise: Oscillator,
    vibrato: Oscillator,
    envelope: Envelope,
    transient: Envelope,
    filter_envelope: Envelope,
    filter: LowPass,
    rng: Rng,
    channel: u8,
    note: Option<u8>,
    frequency: f32,
    detune_ratio: f32,
    amplitude: f32,
    /// Note-off arrived but the sustain pedal is holding this voice.
    sustained: bool,
    transient_level: f32,
    vibrato_depth: f32,
    vibrato_age_s: f32,
    body_level: f32,
    body_decay: f32,
    noise_mix: f32,
    pitch_offset: f32,
    pitch_decay: f32,
    cutoff_hz: f32,
    cutoff_range_hz: f32,
}

impl Voice {
    pub fn new(sample_rate: f32, seed: u32) -> Self {
        Self {
            sample_rate,
            oscillator: Oscillator::new(sample_rate, Waveform::Triangle),
            detuned: Oscillator::new(sample_rate, Waveform::Triangle),
            noise: Oscillator::new(sample_rate, Waveform::Noise),
            vibrato: Oscillator::new(sample_rate, Waveform::Sine),
            envelope: Envelope::new(sample_rate),
            transient: Envelope::new(sample_rate),
            filter_envelope: Envelope::new(sample_rate),
            filter: LowPass::new(sample_rate),
            rng: Rng::new(seed),
            channel: 0,
            note: None,
            frequency: 440.0,
            detune_ratio: DETUNE,
            amplitude: 0.0,
            sustained: false,
            transient_level: 0.0,
            vibrato_depth: 0.0,
            vibrato_age_s: 0.0,
            body_level: 1.0,
            body_decay: 1.0,
            noise_mix: 0.0,
            pitch_offset: 0.0,
            pitch_decay: 0.0,
            cutoff_hz: 1000.0,
            cutoff_range_hz: 0.0,
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
        self.sustained = false;

        let velocity = velocity as f32 / 127.0;
        self.amplitude = velocity * (1.0 + self.rng.next_bipolar() * LEVEL_JITTER);

        // Percussion tunes itself: the note picked the sound, not the pitch.
        self.frequency = instrument.fixed_pitch.unwrap_or_else(|| note_to_freq(note));
        self.detune_ratio = DETUNE * (1.0 + self.rng.next_bipolar() * DETUNE_JITTER);
        self.oscillator.set_waveform(instrument.waveform);
        self.detuned.set_waveform(instrument.waveform);
        self.oscillator.set_phase(self.rng.next_f32());
        self.detuned.set_phase(self.rng.next_f32());

        // Harder playing is brighter, not merely louder.
        let velocity_brightness = 0.4 + 0.6 * velocity;
        self.cutoff_hz = self.frequency * instrument.cutoff_ratio;
        self.cutoff_range_hz = self.cutoff_hz * instrument.brightness * velocity_brightness;
        self.filter.reset();

        self.noise_mix = instrument.noise_mix;
        self.pitch_offset = instrument.pitch_drop;
        self.pitch_decay = (-1.0 / (instrument.pitch_drop_s * self.sample_rate)).exp();

        self.transient_level = instrument.transient * velocity;
        self.transient
            .configure(0.0005, instrument.transient_s, 0.0, 0.01);
        self.transient.trigger();

        self.filter_envelope.configure(
            instrument.attack_s,
            instrument.brightness_decay_s,
            0.0,
            0.1,
        );
        self.filter_envelope.trigger();

        self.vibrato_depth = instrument.vibrato_depth;
        self.vibrato_age_s = 0.0;
        self.vibrato.set_frequency(VIBRATO_HZ);
        self.vibrato.set_phase(self.rng.next_f32());

        self.body_level = 1.0;
        self.body_decay = (-1.0 / (instrument.body_decay_s * self.sample_rate)).exp();

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
        self.transient.reset();
        self.filter_envelope.reset();
        self.filter.reset();
        self.note = None;
        self.sustained = false;
    }

    pub fn next_sample(&mut self) -> f32 {
        let vibrato = if self.vibrato_depth > 0.0 {
            self.vibrato_age_s += 1.0 / self.sample_rate;
            let onset = (self.vibrato_age_s / VIBRATO_ONSET_S).min(1.0);
            self.vibrato.next_sample() * self.vibrato_depth * onset
        } else {
            0.0
        };
        self.pitch_offset *= self.pitch_decay;
        let frequency = self.frequency * (1.0 + vibrato) * semitones(self.pitch_offset);
        self.oscillator.set_frequency(frequency);
        self.detuned.set_frequency(frequency * self.detune_ratio);

        let pitched = (self.oscillator.next_sample() + self.detuned.next_sample() * DETUNE_MIX)
            / (1.0 + DETUNE_MIX);
        let noise = self.noise.next_sample();
        let tone = pitched * (1.0 - self.noise_mix) + noise * self.noise_mix;
        let transient = noise * self.transient.next_sample() * self.transient_level;

        // The filter envelope closes over the note, so it starts bright and darkens.
        self.filter
            .set_cutoff(self.cutoff_hz + self.cutoff_range_hz * self.filter_envelope.next_sample());
        let filtered = self.filter.process(tone + transient);

        self.body_level *= self.body_decay;
        filtered * self.envelope.next_sample() * self.amplitude * self.body_level
    }
}

fn semitones(offset: f32) -> f32 {
    if offset == 0.0 {
        1.0
    } else {
        2.0f32.powf(offset / 12.0)
    }
}

fn note_to_freq(note: u8) -> f32 {
    440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0)
}
