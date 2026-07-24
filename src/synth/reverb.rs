/// Freeverb: parallel damped comb filters feeding series allpasses. The right
/// channel runs the same delays a little longer, which decorrelates the two
/// sides into a stereo field.
const COMB_LENGTHS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_LENGTHS: [usize; 4] = [556, 441, 341, 225];
const STEREO_SPREAD: usize = 23;
const REFERENCE_RATE: f32 = 44_100.0;

const FEEDBACK: f32 = 0.86;
const DAMPING: f32 = 0.32;
/// Each comb resonates to 1/(1 - FEEDBACK) times its input, so the input is
/// scaled by the inverse to keep the wet signal at roughly unity.
const INPUT_GAIN: f32 = 1.0 - FEEDBACK;

struct Delay {
    buffer: Vec<f32>,
    index: usize,
}

impl Delay {
    fn new(length: usize) -> Self {
        Self {
            buffer: vec![0.0; length.max(1)],
            index: 0,
        }
    }

    fn advance(&mut self, value: f32) -> f32 {
        let out = self.buffer[self.index];
        self.buffer[self.index] = value;
        self.index = (self.index + 1) % self.buffer.len();
        out
    }
}

struct Comb {
    delay: Delay,
    filter_store: f32,
}

impl Comb {
    fn process(&mut self, input: f32) -> f32 {
        let out = self.delay.buffer[self.delay.index];
        self.filter_store = out * (1.0 - DAMPING) + self.filter_store * DAMPING;
        self.delay.advance(input + self.filter_store * FEEDBACK);
        out
    }
}

struct Allpass {
    delay: Delay,
}

impl Allpass {
    fn process(&mut self, input: f32) -> f32 {
        let buffered = self.delay.buffer[self.delay.index];
        self.delay.advance(input + buffered * 0.5);
        buffered - input
    }
}

struct Channel {
    combs: Vec<Comb>,
    allpasses: Vec<Allpass>,
}

impl Channel {
    fn new(sample_rate: f32, offset: usize) -> Self {
        let scale = |length: usize| {
            ((length + offset) as f32 * sample_rate / REFERENCE_RATE).round() as usize
        };
        Self {
            combs: COMB_LENGTHS
                .iter()
                .map(|&length| Comb {
                    delay: Delay::new(scale(length)),
                    filter_store: 0.0,
                })
                .collect(),
            allpasses: ALLPASS_LENGTHS
                .iter()
                .map(|&length| Allpass {
                    delay: Delay::new(scale(length)),
                })
                .collect(),
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let mut out: f32 = self.combs.iter_mut().map(|comb| comb.process(input)).sum();
        out /= self.combs.len() as f32;
        for allpass in &mut self.allpasses {
            out = allpass.process(out);
        }
        out
    }
}

pub struct Reverb {
    left: Channel,
    right: Channel,
}

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            left: Channel::new(sample_rate, 0),
            right: Channel::new(sample_rate, STEREO_SPREAD),
        }
    }

    pub fn process(&mut self, input: f32) -> (f32, f32) {
        let input = input * INPUT_GAIN;
        (self.left.process(input), self.right.process(input))
    }
}
