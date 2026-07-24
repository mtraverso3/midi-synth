pub mod engine;
pub mod limiter;
pub mod render;
pub mod sequencer;
pub mod smf;
pub mod soundfont;
pub mod synth;
pub mod viz;

/// Silence rendered after the last event so release tails aren't clipped.
pub const TAIL_SECONDS: f64 = 2.0;
