use anyhow::{anyhow, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat,
};
use hound::{WavSpec, WavWriter};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

/// Records microphone audio to a WAV file.
/// Blocks (on a background thread) until stop_signal is set.
pub fn run_audio_capture(output_path: String, stop_signal: Arc<AtomicBool>) -> Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow!("No default input audio device found"))?;

    log::info!("Audio device: {}", device.name().unwrap_or_default());

    let config = device
        .default_input_config()
        .map_err(|e| anyhow!("Failed to get audio config: {}", e))?;

    let sample_rate = config.sample_rate().0;
    let channels = config.channels();

    log::info!(
        "Audio capture: {}Hz, {} channels → {}",
        sample_rate,
        channels,
        output_path
    );

    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let writer = Arc::new(Mutex::new(
        WavWriter::create(&output_path, spec)
            .map_err(|e| anyhow!("Failed to create WAV writer: {}", e))?,
    ));

    let writer_clone = writer.clone();
    let err_fn = |err| log::error!("Audio stream error: {}", err);

    let stream = match config.sample_format() {
        SampleFormat::I16 => {
            let w = writer_clone.clone();
            device.build_input_stream(
                &config.into(),
                move |data: &[i16], _| {
                    if let Ok(mut wtr) = w.lock() {
                        for &sample in data {
                            wtr.write_sample(sample).ok();
                        }
                    }
                },
                err_fn,
                None,
            )?
        }
        SampleFormat::F32 => {
            let w = writer_clone.clone();
            device.build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    if let Ok(mut wtr) = w.lock() {
                        for &sample in data {
                            let s = (sample * i16::MAX as f32)
                                .clamp(i16::MIN as f32, i16::MAX as f32)
                                as i16;
                            wtr.write_sample(s).ok();
                        }
                    }
                },
                err_fn,
                None,
            )?
        }
        SampleFormat::U8 => {
            let w = writer_clone.clone();
            device.build_input_stream(
                &config.into(),
                move |data: &[u8], _| {
                    if let Ok(mut wtr) = w.lock() {
                        for &sample in data {
                            let s = (sample as i16 - 128) * 256;
                            wtr.write_sample(s).ok();
                        }
                    }
                },
                err_fn,
                None,
            )?
        }
        fmt => return Err(anyhow!("Unsupported audio sample format: {:?}", fmt)),
    };

    stream
        .play()
        .map_err(|e| anyhow!("Failed to start audio stream: {}", e))?;

    // Poll the stop signal at 100ms intervals
    while !stop_signal.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    drop(stream);

    // Finalize the WAV file
    if let Ok(wtr) = Arc::try_unwrap(writer) {
        if let Ok(w) = wtr.into_inner() {
            w.finalize().ok();
        }
    }

    log::info!("Audio capture finished");
    Ok(())
}
