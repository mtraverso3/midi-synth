# midi

A MIDI file player with a built-in software synthesizer. 
It parses a `.mid` file, synthesizes the notes from scratch 
(oscillators, ADSR envelopes, a low-pass filter), 
and plays them out loud or writes them to an audio file.

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
cargo run -- --help
```

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
