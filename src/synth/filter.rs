/// Two-pole state-variable low-pass (topology-preserving transform). Steeper
/// and resonant, unlike a one-pole, so sweeping the cutoff is actually audible.
pub struct LowPass {
    sample_rate: f32,
    g: f32,
    k: f32,
    ic1: f32,
    ic2: f32,
}

const RESONANCE: f32 = 0.9;

impl LowPass {
    pub fn new(sample_rate: f32) -> Self {
        let mut filter = Self {
            sample_rate,
            g: 1.0,
            k: 1.0 / RESONANCE,
            ic1: 0.0,
            ic2: 0.0,
        };
        filter.set_cutoff(sample_rate * 0.25);
        filter
    }

    /// Scale the resonance relative to the filter's natural Q.
    pub fn set_resonance(&mut self, scale: f32) {
        self.k = 1.0 / (RESONANCE * scale).clamp(0.3, 8.0);
    }

    pub fn reset(&mut self) {
        self.ic1 = 0.0;
        self.ic2 = 0.0;
    }

    pub fn set_cutoff(&mut self, cutoff_hz: f32) {
        let nyquist = self.sample_rate * 0.5;
        let cutoff = cutoff_hz.clamp(20.0, nyquist * 0.98);
        self.g = (std::f32::consts::PI * cutoff / self.sample_rate).tan();
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let a1 = 1.0 / (1.0 + self.g * (self.g + self.k));
        let a2 = self.g * a1;
        let a3 = self.g * a2;

        let v3 = input - self.ic2;
        let v1 = a1 * self.ic1 + a2 * v3;
        let v2 = self.ic2 + a2 * self.ic1 + a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        v2
    }
}
