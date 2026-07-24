#[derive(Clone, Copy, PartialEq)]
enum Stage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// How close to a target level counts as "arrived", ending a stage.
const EPSILON: f32 = 0.001;

pub struct Envelope {
    sample_rate: f32,
    stage: Stage,
    level: f32,
    attack_coeff: f32,
    decay_coeff: f32,
    sustain_level: f32,
    release_coeff: f32,
}

impl Envelope {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            stage: Stage::Idle,
            level: 0.0,
            attack_coeff: 1.0,
            decay_coeff: 1.0,
            sustain_level: 1.0,
            release_coeff: 1.0,
        }
    }

    pub fn configure(&mut self, attack_s: f32, decay_s: f32, sustain_level: f32, release_s: f32) {
        // One-pole smoothing coefficient: each sample moves this fraction of the
        // remaining distance to the target, giving an exponential curve.
        let coeff = |seconds: f32| {
            if seconds <= 0.0 {
                1.0
            } else {
                1.0 - (-1.0 / (seconds * self.sample_rate)).exp()
            }
        };
        self.attack_coeff = coeff(attack_s);
        self.decay_coeff = coeff(decay_s);
        self.sustain_level = sustain_level;
        self.release_coeff = coeff(release_s);
    }

    pub fn trigger(&mut self) {
        self.stage = Stage::Attack;
    }

    pub fn release(&mut self) {
        self.stage = Stage::Release;
    }

    pub fn is_finished(&self) -> bool {
        self.stage == Stage::Idle
    }

    pub fn level(&self) -> f32 {
        self.level
    }

    pub fn next_sample(&mut self) -> f32 {
        match self.stage {
            Stage::Idle => self.level = 0.0,
            Stage::Attack => {
                self.level += (1.0 - self.level) * self.attack_coeff;
                if self.level >= 1.0 - EPSILON {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                self.level += (self.sustain_level - self.level) * self.decay_coeff;
                if (self.level - self.sustain_level).abs() < EPSILON {
                    self.level = self.sustain_level;
                    self.stage = Stage::Sustain;
                }
            }
            // A note whose sustain is silence (plucked/percussive) has fully
            // faded, so free the voice instead of holding at zero forever.
            Stage::Sustain if self.sustain_level < EPSILON => self.stage = Stage::Idle,
            Stage::Sustain => self.level = self.sustain_level,
            Stage::Release => {
                self.level -= self.level * self.release_coeff;
                if self.level < EPSILON {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
                }
            }
        }
        self.level
    }
}
