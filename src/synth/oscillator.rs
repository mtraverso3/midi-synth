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
    rng: u32,
}

impl Oscillator {
    pub fn new(sample_rate: f32, waveform: Waveform) -> Self {
        Self {
            sample_rate,
            waveform,
            phase: 0.0,
            phase_step: 0.0,
            rng: 0x2545_f491,
        }
    }

    pub fn set_frequency(&mut self, freq: f32) {
        self.phase_step = freq / self.sample_rate;
    }

    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform;
    }

    pub fn next_sample(&mut self) -> f32 {
        let value = match self.waveform {
            Waveform::Sine => (self.phase * std::f32::consts::TAU).sin(),
            Waveform::Saw => 2.0 * self.phase - 1.0,
            Waveform::Square => {
                if self.phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Triangle => {
                if self.phase < 0.5 {
                    4.0 * self.phase - 1.0
                } else {
                    3.0 - 4.0 * self.phase
                }
            }
            Waveform::Noise => self.next_noise(),
        };
        self.phase = (self.phase + self.phase_step).fract();
        value
    }

    /// White noise via a fast xorshift PRNG mapped to -1.0..1.0.
    fn next_noise(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}
