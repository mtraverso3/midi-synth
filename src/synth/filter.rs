/// One-pole low-pass filter: eases toward the input, smoothing away the harsh
/// high-frequency edges of saw and square waves.
pub struct LowPass {
    alpha: f32,
    prev: f32,
}

impl LowPass {
    pub fn new() -> Self {
        Self {
            alpha: 1.0,
            prev: 0.0,
        }
    }

    pub fn set_cutoff(&mut self, sample_rate: f32, cutoff_hz: f32) {
        let dt = 1.0 / sample_rate;
        let rc = 1.0 / (std::f32::consts::TAU * cutoff_hz);
        self.alpha = dt / (rc + dt);
    }

    pub fn process(&mut self, input: f32) -> f32 {
        self.prev += self.alpha * (input - self.prev);
        self.prev
    }
}
