use std::path::Path;

type Error = Box<dyn std::error::Error>;

/// Write interleaved stereo f32 samples to a file, choosing the encoder from
/// the extension.
pub fn write(path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), Error> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("wav") => write_wav(path, samples, sample_rate),
        Some("mp3") => write_mp3(path, samples, sample_rate),
        _ => Err("output file must end in .wav or .mp3".into()),
    }
}

fn to_i16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect()
}

fn write_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), Error> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for sample in to_i16(samples) {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    Ok(())
}

fn write_mp3(path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), Error> {
    use mp3lame_encoder::{Bitrate, Builder, FlushNoGap, InterleavedPcm, Quality};

    let mut builder = Builder::new().ok_or("failed to create MP3 encoder")?;
    builder.set_num_channels(2).map_err(|e| e.to_string())?;
    builder
        .set_sample_rate(sample_rate)
        .map_err(|e| e.to_string())?;
    builder
        .set_brate(Bitrate::Kbps192)
        .map_err(|e| e.to_string())?;
    builder
        .set_quality(Quality::Best)
        .map_err(|e| e.to_string())?;
    let mut encoder = builder.build().map_err(|e| e.to_string())?;

    let pcm = to_i16(samples);
    // The flush needs room of its own beyond what encoding the samples requires.
    const FLUSH_HEADROOM: usize = 7200;
    let mut mp3 =
        Vec::with_capacity(mp3lame_encoder::max_required_buffer_size(pcm.len()) + FLUSH_HEADROOM);

    let encoded = encoder
        .encode(InterleavedPcm(&pcm), mp3.spare_capacity_mut())
        .map_err(|e| e.to_string())?;
    unsafe { mp3.set_len(mp3.len() + encoded) };

    let flushed = encoder
        .flush::<FlushNoGap>(mp3.spare_capacity_mut())
        .map_err(|e| e.to_string())?;
    unsafe { mp3.set_len(mp3.len() + flushed) };

    std::fs::write(path, &mp3)?;
    Ok(())
}
