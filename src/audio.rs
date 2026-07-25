use std::sync::mpsc::{Receiver, Sender, channel};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, Stream};

use midi::engine::{self, Command};
use midi::soundfont::SoundFont;

type Error = Box<dyn std::error::Error>;

pub fn build_stream(
    soundfont: Option<SoundFont>,
    gain: f32,
) -> Result<(Stream, Sender<Command>), Error> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("no audio output device available")?;

    // Ask for the device's current configuration. Picking one out of
    // `supported_output_configs` instead can select a Bluetooth headset's
    // hands-free rate, which switches it into call mode and opens its mic.
    let supported_config = device.default_output_config()?;

    let sample_format = supported_config.sample_format();
    let config = supported_config.into();

    let (tx, rx) = channel();

    let stream = match sample_format {
        SampleFormat::F32 => run::<f32>(&device, config, rx, soundfont, gain)?,
        SampleFormat::I16 => run::<i16>(&device, config, rx, soundfont, gain)?,
        SampleFormat::U16 => run::<u16>(&device, config, rx, soundfont, gain)?,
        other => return Err(format!("unsupported sample format '{other}'").into()),
    };

    Ok((stream, tx))
}

fn run<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    rx: Receiver<Command>,
    soundfont: Option<SoundFont>,
    gain: f32,
) -> Result<Stream, Error>
where
    T: Sample + SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    let mut engine = engine::build(soundfont, config.sample_rate, gain)?;
    let mut scratch: Vec<f32> = Vec::new();

    let err_fn = |err| eprintln!("audio stream error: {err}");

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                while let Ok(command) = rx.try_recv() {
                    engine.handle(command);
                }
                scratch.resize(data.len(), 0.0);
                engine.fill(&mut scratch, channels);
                for (out, sample) in data.iter_mut().zip(&scratch) {
                    *out = T::from_sample(*sample);
                }
            },
            err_fn,
            None,
        )?;

    stream.play()?;
    Ok(stream)
}
