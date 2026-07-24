#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum Waveform {
    Sine,
    Saw,
    Square,
    Triangle,
}

impl Waveform {
    fn sample(self, phase: f32) -> f32 {
        match self {
            Waveform::Sine => (phase * std::f32::consts::TAU).sin(),
            Waveform::Saw => 2.0 * phase - 1.0,
            Waveform::Square => {
                if phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Triangle => {
                if phase < 0.5 {
                    4.0 * phase - 1.0
                } else {
                    3.0 - 4.0 * phase
                }
            }
        }
    }
}

pub struct Oscillator {
    sample_rate: f32,
    waveform: Waveform,
    phase: f32,
    phase_step: f32,
}

impl Oscillator {
    pub fn new(sample_rate: f32, waveform: Waveform) -> Self {
        Self {
            sample_rate,
            waveform,
            phase: 0.0,
            phase_step: 0.0,
        }
    }

    pub fn set_frequency(&mut self, freq: f32) {
        self.phase_step = freq / self.sample_rate;
    }

    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform;
    }

    pub fn next_sample(&mut self) -> f32 {
        let value = self.waveform.sample(self.phase);
        self.phase = (self.phase + self.phase_step).fract();
        value
    }
}
