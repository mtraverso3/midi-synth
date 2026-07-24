use crate::TAIL_SECONDS;
use crate::engine::Engine;
use crate::sequencer::to_command;
use crate::smf::Event;

/// Samples rendered per block between events.
const CHUNK: usize = 512;

/// Render events to a mono f32 buffer, faster than real time, by filling the
/// engine up to each event's sample position and applying it.
pub fn render(events: &[Event], sample_rate: u32, mut engine: Box<dyn Engine>) -> Vec<f32> {
    let total = events.last().map(|e| e.time_s).unwrap_or(0.0) + TAIL_SECONDS;
    let mut out = Vec::with_capacity((total * sample_rate as f64) as usize);
    let mut scratch = [0.0f32; CHUNK];

    let mut produced = 0u64;
    let mut render_until = |out: &mut Vec<f32>, engine: &mut Box<dyn Engine>, target: u64| {
        while produced < target {
            let n = ((target - produced) as usize).min(CHUNK);
            engine.fill(&mut scratch[..n], 1);
            out.extend_from_slice(&scratch[..n]);
            produced += n as u64;
        }
    };

    for event in events {
        let target = (event.time_s * sample_rate as f64) as u64;
        render_until(&mut out, &mut engine, target);
        engine.handle(to_command(event));
    }
    render_until(&mut out, &mut engine, (total * sample_rate as f64) as u64);

    out
}
