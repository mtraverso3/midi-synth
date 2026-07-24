use crate::TAIL_SECONDS;
use crate::midi::Event;
use crate::sequencer::to_command;
use crate::synth::Synth;

/// Render events to a mono f32 buffer, faster than real time, by advancing the
/// synth to each event's sample position and applying it.
pub fn render(events: &[Event], sample_rate: u32) -> Vec<f32> {
    let mut synth = Synth::new(sample_rate as f32);
    let total = events.last().map(|e| e.time_s).unwrap_or(0.0) + TAIL_SECONDS;
    let mut out = Vec::with_capacity((total * sample_rate as f64) as usize);

    let mut sample_index = 0u64;
    for event in events {
        let target = (event.time_s * sample_rate as f64) as u64;
        while sample_index < target {
            out.push(synth.next_sample());
            sample_index += 1;
        }
        synth.handle(to_command(event));
    }

    for _ in 0..(TAIL_SECONDS * sample_rate as f64) as u64 {
        out.push(synth.next_sample());
    }
    out
}
