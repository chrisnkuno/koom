use anyhow::{anyhow, Result};
use nokhwa::{
    pixel_format::RgbFormat,
    utils::{CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType},
    Camera,
};
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

/// Spawns an FFmpeg process that reads raw RGB24 frames from stdin
/// and writes an H.264 MP4 for the webcam feed.
pub fn spawn_ffmpeg_webcam(width: u32, height: u32, fps: u32, output_path: &str) -> Result<Child> {
    let child = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "rawvideo",
            "-pixel_format",
            "rgb24",
            "-video_size",
            &format!("{}x{}", width, height),
            "-framerate",
            &fps.to_string(),
            "-i",
            "-",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-preset",
            "ultrafast",
            "-tune",
            "zerolatency",
            "-crf",
            "23",
            output_path,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(child)
}

/// Runs the webcam capture loop: grabs RGB frames from the webcam
/// and pipes them into FFmpeg stdin.
pub fn run_webcam_capture(
    webcam_index: usize,
    fps: u32,
    output_path: String,
    stop_signal: Arc<AtomicBool>,
) -> Result<()> {
    let index = CameraIndex::Index(webcam_index as u32);
    let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(
        CameraFormat::new_from(1280, 720, FrameFormat::MJPEG, fps),
    ));

    let mut camera = Camera::new(index, requested)
        .map_err(|e| anyhow!("Failed to open webcam {}: {}", webcam_index, e))?;

    let fmt = camera.camera_format();
    let width = fmt.width();
    let height = fmt.height();

    log::info!(
        "Webcam capture: {}x{} @ {}fps → {}",
        width,
        height,
        fps,
        output_path
    );

    camera
        .open_stream()
        .map_err(|e| anyhow!("Failed to open webcam stream: {}", e))?;

    let mut child = spawn_ffmpeg_webcam(width, height, fps, &output_path)?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("Could not get FFmpeg stdin for webcam"))?;

    let frame_duration = Duration::from_secs_f64(1.0 / fps as f64);
    let mut next_frame = Instant::now();

    while !stop_signal.load(Ordering::Relaxed) {
        let now = Instant::now();
        if now < next_frame {
            std::thread::sleep(next_frame - now);
        }
        next_frame += frame_duration;

        match camera.frame() {
            Ok(buf) => match buf.decode_image::<RgbFormat>() {
                Ok(img) => {
                    let raw: &[u8] = img.as_raw();
                    if let Err(e) = stdin.write_all(raw) {
                        log::warn!("Webcam FFmpeg stdin write error: {}", e);
                        break;
                    }
                }
                Err(e) => log::warn!("Webcam frame decode error: {}", e),
            },
            Err(e) => log::warn!("Webcam frame grab error: {}", e),
        }
    }

    drop(stdin);
    let _ = child.wait();
    camera.stop_stream().ok();
    log::info!("Webcam capture finished");
    Ok(())
}
