# midi

A MIDI file player with a built-in software synthesizer.
It parses a `.mid` file, synthesizes the notes from scratch
(anti-aliased oscillators, ADSR amplitude and filter envelopes, a resonant
low-pass filter, and a reverb), and plays them out loud or writes them to an
audio file. Optionally plays through a SoundFont (`.sf2`) for sampled instruments.

## Requirements

- Rust (stable)
- `libmp3lame` is built from source automatically; MP3 output needs a C
  compiler (`clang`/`gcc`) on your `PATH`.

## Usage

```sh
cargo run -- <FILE.mid>          # play a MIDI file
cargo run                        # play the bundled demo
cargo run -- song.mid --tui      # play with a live terminal visualizer
cargo run -- song.mid -o out.mp3 # render to out.mp3 (or out.wav)
cargo run -- song.mid -v         # print each note event as it plays
cargo run -- song.mid -s gm.sf2  # play through a SoundFont
cargo run -- --help
```

### Instruments

The built-in synth covers all 128 General MIDI programs and the percussion kit.
Each voice is an [`Instrument`](src/synth/instrument/mod.rs) — a plain parameter
set describing an oscillator, two envelopes, a filter and its movement, noise
content, and pitch behaviour.

Instruments are built from a small set of archetypes (`STRUCK`, `PLUCKED`,
`BOWED`, `BLOWN`, `REED`, `BRASS`, `ORGAN`, `PAD`, `LEAD`, `BELL`, `HIT`), and
each program overrides only what makes it distinctive:

```rust
25 => Instrument { waveform: Waveform::Saw, decay_s: 1.3, brightness: 13.0, ..PLUCKED }
```

To retune an instrument, edit its line in [`melodic.rs`](src/synth/instrument/melodic.rs)
(by program) or [`percussion.rs`](src/synth/instrument/percussion.rs) (by note).

### SoundFonts

Pass any General MIDI `.sf2` with `-s`/`--soundfont` to use sampled instruments
instead of the built-in synth. It works for live playback and file rendering.
Free options include FluidR3_GM and GeneralUser GS.

### TUI controls

- `space` — pause / resume
- `←` / `→` — seek ±5s
- `q` / `Esc` — quit

## Test files

The bundled `assets/*.mid` are generated:

```sh
cargo run --example make_demo_midi   # multi-instrument demo
cargo run --example make_test_midi   # Twinkle Twinkle
```
