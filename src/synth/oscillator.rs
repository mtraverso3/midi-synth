use super::rng::Rng;

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum Waveform {
    Sine,
    Saw,
    Square,
    Triangle,
    Noise,
}

pub struct Oscillator {
    sample_rate: f32,
    waveform: Waveform,
    phase: f32,
    phase_step: f32,
    rng: Rng,
}

impl Oscillator {
    pub fn new(sample_rate: f32, waveform: Waveform) -> Self {
        Self {
            sample_rate,
            waveform,
            phase: 0.0,
            phase_step: 0.0,
            rng: Rng::new(0x2545_f491),
        }
    }

    pub fn set_frequency(&mut self, freq: f32) {
        self.phase_step = freq / self.sample_rate;
    }

    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform;
    }

    pub fn set_phase(&mut self, phase: f32) {
        self.phase = phase.fract();
    }

    pub fn next_sample(&mut self) -> f32 {
        let value = match self.waveform {
            Waveform::Sine => (self.phase * std::f32::consts::TAU).sin(),
            Waveform::Saw => 2.0 * self.phase - 1.0 - self.blep(self.phase),
            Waveform::Square => {
                let raw = if self.phase < 0.5 { 1.0 } else { -1.0 };
                raw - self.blep(self.phase) + self.blep((self.phase + 0.5).fract())
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
        value
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
