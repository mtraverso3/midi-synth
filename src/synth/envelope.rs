#[derive(Clone, Copy, PartialEq)]
enum Stage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

pub struct Envelope {
    stage: Stage,
    level: f32,
    attack_rate: f32,
    decay_rate: f32,
    sustain_level: f32,
    release_rate: f32,
}

impl Envelope {
    pub fn new(
        sample_rate: f32,
        attack_s: f32,
        decay_s: f32,
        sustain_level: f32,
        release_s: f32,
    ) -> Self {
        let per_sample = |seconds: f32| {
            if seconds <= 0.0 {
                1.0
            } else {
                1.0 / (seconds * sample_rate)
            }
        };

        Self {
            stage: Stage::Idle,
            level: 0.0,
            attack_rate: per_sample(attack_s),
            decay_rate: per_sample(decay_s),
            sustain_level,
            release_rate: per_sample(release_s),
        }
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

    pub fn next_sample(&mut self) -> f32 {
        match self.stage {
            Stage::Idle => self.level = 0.0,
            Stage::Attack => {
                self.level += self.attack_rate;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                self.level -= self.decay_rate;
                if self.level <= self.sustain_level {
                    self.level = self.sustain_level;
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => self.level = self.sustain_level,
            Stage::Release => {
                self.level -= self.release_rate;
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
                }
            }
        }
        self.level
    }
}
