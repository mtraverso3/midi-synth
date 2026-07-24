# midi

A MIDI file player with a built-in software synthesizer.
It parses a `.mid` file, synthesizes the notes from scratch
(anti-aliased oscillators, ADSR amplitude and filter envelopes, a resonant
low-pass filter, and a reverb), and plays them out loud or writes them to an
audio file in stereo. Optionally plays through a SoundFont (`.sf2`) for sampled
instruments.

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
cargo run -- song.mid -g -6      # trim the output by 6 dB
cargo run -- --help
```

### Levels

Notes, program changes, pitch bend (with its RPN 0 range), channel pressure and
the controllers below are followed; the parser hands every channel voice message
to the engine, so a SoundFont receives even the ones the built-in synth ignores.

| | |
|---|---|
| CC1 | modulation |
| CC7 / CC11 | volume, expression |
| CC10 | pan |
| CC64 / CC66 / CC67 | sustain, sostenuto, soft pedal |
| CC0 / CC32 | bank select |
| CC6 / CC38 / CC96 / CC97 | data entry, for RPN 0 |
| CC71-74 | resonance, release, attack, brightness |
| CC120-127 | channel mode: all sound off, all notes off, reset controllers |

Universal system exclusive master volume and "GM System On" are acted on; other
system exclusive messages are device-specific and ignored.

Polyphonic key pressure leans on the vibrato of the single note it names.
Velocity sets attenuation by the square law the SoundFont and DLS specs use, so
a part written quiet stays quiet against the rest of the mix.

SMF formats 0 and 1 are supported, with either metrical or SMPTE timecode
division. Format 2 holds independent sequences rather than one piece, so it is
rejected rather than played stacked.

The master bus ends in a look-ahead limiter, so polyphony sets the density of
the music rather than its volume: a dense passage is turned down as a whole
instead of clipping, and the output stays under -0.4 dBFS whether one note is
sounding or two hundred. `-g`/`--gain` trims the level going into it.

### Instruments

The built-in synth covers all 128 General MIDI programs and the percussion kit.
Each voice is an [`Instrument`](src/synth/instrument/mod.rs) — a plain parameter
set describing an oscillator, two envelopes, a filter and its movement, noise
content, and pitch behaviour.

Waveforms are scaled to equal power, so choosing a shape for a patch changes its
colour rather than its loudness, and each instrument carries a `level` trim for
balancing it against the rest of the set.

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
