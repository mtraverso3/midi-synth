/// Ceiling the output is held under.
const CEILING: f32 = 0.95;
/// How far ahead the limiter sees. The signal is delayed by this much, so the
/// gain is already down by the time a peak reaches the output.
const LOOKAHEAD_MS: f32 = 3.0;
/// How fast the gain comes back up once the loud passage has gone: slow enough
/// not to pump on every note, fast enough to recover between phrases.
const RELEASE_MS: f32 = 150.0;

/// Look-ahead peak limiter. Dense passages are turned down as a whole rather
/// than clipped sample by sample, which keeps the waveform shape instead of
/// trading it for distortion.
pub struct Limiter {
    buffer: Vec<f32>,
    /// The gain each buffered frame will need when it reaches the output.
    needed: Vec<f32>,
    channels: usize,
    frames: usize,
    index: usize,
    gain: f32,
    release: f32,
}

impl Limiter {
    pub fn new(sample_rate: u32) -> Self {
        let sample_rate = sample_rate as f32;
        let frames = ((LOOKAHEAD_MS / 1000.0) * sample_rate).round().max(1.0) as usize;
        Self {
            buffer: Vec::new(),
            needed: vec![1.0; frames],
            channels: 0,
            frames,
            index: 0,
            gain: 1.0,
            release: coefficient(RELEASE_MS, sample_rate),
        }
    }

    pub fn process(&mut self, data: &mut [f32], channels: usize) {
        if channels == 0 {
            return;
        }
        if channels != self.channels {
            self.channels = channels;
            self.buffer = vec![0.0; self.frames * channels];
            self.needed.fill(1.0);
            self.index = 0;
        }

        for frame in data.chunks_mut(channels) {
            let peak = frame.iter().fold(0.0f32, |peak, s| peak.max(s.abs()));
            self.needed[self.index] = if peak > CEILING { CEILING / peak } else { 1.0 };

            // The lowest gain anything still in the buffer will need. Reaching
            // it immediately is what keeps peaks from escaping: the drop lands
            // on the quieter samples already in flight, ahead of the loud one.
            let needed = self.needed.iter().fold(f32::MAX, |low, g| low.min(*g));
            self.gain += (needed - self.gain) * self.release;
            self.gain = self.gain.min(needed);

            let slot = self.index * channels;
            for (offset, sample) in frame.iter_mut().enumerate() {
                let delayed = self.buffer[slot + offset];
                self.buffer[slot + offset] = *sample;
                *sample = (delayed * self.gain).clamp(-1.0, 1.0);
            }
            self.index = (self.index + 1) % self.frames;
        }
    }
}

fn coefficient(milliseconds: f32, sample_rate: f32) -> f32 {
    1.0 - (-1.0 / (milliseconds / 1000.0 * sample_rate)).exp()
}
