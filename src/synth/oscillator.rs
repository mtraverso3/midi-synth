use super::rng::Rng;

#[derive(Clone, Copy)]
pub enum Waveform {
    Sine,
    Saw,
    /// Rectangle wave; the duty cycle sets how nasal or hollow it sounds.
    /// A duty of 0.5 is a square wave.
    Pulse(f32),
    Triangle,
    Noise,
}

pub const SQUARE: Waveform = Waveform::Pulse(0.5);

/// The RMS of a saw or triangle at peak 1.0. Every waveform is scaled to match
/// it, so choosing a shape for a patch changes its colour and not its loudness:
/// a square carries 1.73x the power of a saw at the same peak.
const REFERENCE_RMS: f32 = 0.577_350_3;

impl Waveform {
    fn level(self) -> f32 {
        match self {
            Waveform::Sine => REFERENCE_RMS * std::f32::consts::SQRT_2,
            Waveform::Saw | Waveform::Triangle | Waveform::Noise => 1.0,
            Waveform::Pulse(duty) => {
                let duty = duty.clamp(MIN_DUTY, 0.5);
                REFERENCE_RMS / (duty / (1.0 - duty)).sqrt()
            }
        }
    }
}

const MIN_DUTY: f32 = 0.05;

pub struct Oscillator {
    sample_rate: f32,
    waveform: Waveform,
    phase: f32,
    phase_step: f32,
    /// Scaling that keeps every waveform at the same power.
    level: f32,
    rng: Rng,
}

impl Oscillator {
    pub fn new(sample_rate: f32, waveform: Waveform) -> Self {
        Self {
            sample_rate,
            waveform,
            phase: 0.0,
            phase_step: 0.0,
            level: waveform.level(),
            rng: Rng::new(0x2545_f491),
        }
    }

    pub fn set_frequency(&mut self, freq: f32) {
        self.phase_step = freq / self.sample_rate;
    }

    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform;
        self.level = waveform.level();
    }

    pub fn set_phase(&mut self, phase: f32) {
        self.phase = phase.fract();
    }

    pub fn next_sample(&mut self) -> f32 {
        let value = match self.waveform {
            Waveform::Sine => (self.phase * std::f32::consts::TAU).sin(),
            Waveform::Saw => 2.0 * self.phase - 1.0 - self.blep(self.phase),
            Waveform::Pulse(duty) => {
                let duty = duty.clamp(MIN_DUTY, 0.5);
                let raw = if self.phase < duty { 1.0 } else { -1.0 };
                let edge = self.blep(self.phase) - self.blep((self.phase - duty).rem_euclid(1.0));
                // Recentre and rescale: a narrow pulse is otherwise offset and hot.
                (raw - (2.0 * duty - 1.0) - edge) / (2.0 - 2.0 * duty)
            }
            Waveform::Triangle => {
                if self.phase < 0.5 {
                    4.0 * self.phase - 1.0
                } else {
                    3.0 - 4.0 * self.phase
                }
            }
            Waveform::Noise => self.rng.next_bipolar(),
        };
        self.phase = (self.phase + self.phase_step).fract();
        value * self.level
    }

    /// PolyBLEP: rounds off the step discontinuity in saw and square waves,
    /// which would otherwise fold back as inharmonic aliasing.
    fn blep(&self, phase: f32) -> f32 {
        let dt = self.phase_step;
        if dt <= 0.0 {
            return 0.0;
        }
        if phase < dt {
            let t = phase / dt;
            2.0 * t - t * t - 1.0
        } else if phase > 1.0 - dt {
            let t = (phase - 1.0) / dt;
            t * t + 2.0 * t + 1.0
        } else {
            0.0
        }
    }
}
