# midi

A MIDI file player with a built-in software synthesizer. It parses a `.mid`
file, synthesizes the notes from scratch, and plays them out loud or writes
them to a `.wav` or `.mp3`. Pass a SoundFont to use sampled instruments instead.

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
cargo run -- song.mid -s gm.sf2  # play through a SoundFont (.sf2)
cargo run -- song.mid -g -6      # trim the output by 6 dB
cargo run -- --help
```

In the TUI, `space` pauses, `←`/`→` seek ±5s, and `q` quits.

## Test files

The bundled `assets/*.mid` are generated:

```sh
cargo run --example make_demo_midi   # multi-instrument demo
cargo run --example make_test_midi   # Twinkle Twinkle
```
