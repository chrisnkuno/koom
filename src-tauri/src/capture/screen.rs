use anyhow::{anyhow, Result};
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use xcap::Monitor;

/// Spawns an FFmpeg process that reads raw RGBA frames from stdin
/// and writes an H.264 MP4 at the given path.
pub fn spawn_ffmpeg_screen(
    width: u32,
    height: u32,
    fps: u32,
    output_path: &str,
) -> Result<Child> {
    let child = Command::new("ffmpeg")
        .args([
            "-y",
            "-f", "rawvideo",
            "-pixel_format", "rgba",
            "-video_size", &format!("{}x{}", width, height),
            "-framerate", &fps.to_string(),
            "-i", "-",
            "-c:v", "libx264",
            "-pix_fmt", "yuv420p",
            "-preset", "ultrafast",
            "-tune", "zerolatency",
            "-crf", "23",
            output_path,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(child)
}

/// Runs the screen capture loop: captures frames from the selected monitor
/// and pipes them into FFmpeg stdin at the target FPS.
pub async fn run_screen_capture(
    monitor_index: usize,
    fps: u32,
    output_path: String,
    stop_signal: Arc<AtomicBool>,
) -> Result<()> {
    // Get the target monitor
    let monitors = Monitor::all().map_err(|e| anyhow!("Failed to list monitors: {}", e))?;
    let monitor = monitors
        .into_iter()
        .nth(monitor_index)
        .ok_or_else(|| anyhow!("Monitor index {} not found", monitor_index))?;

    let width = monitor.width().unwrap_or(1920);
    let height = monitor.height().unwrap_or(1080);

    log::info!(
        "Screen capture: {}x{} @ {}fps → {}",
        width,
        height,
        fps,
        output_path
    );

    let mut child = spawn_ffmpeg_screen(width, height, fps, &output_path)?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("Could not get FFmpeg stdin"))?;

    let frame_duration = Duration::from_secs_f64(1.0 / fps as f64);
    let mut next_frame = Instant::now();

    while !stop_signal.load(Ordering::Relaxed) {
        let now = Instant::now();
        if now < next_frame {
            tokio::time::sleep(next_frame - now).await;
        }
        next_frame += frame_duration;

        // Capture a frame from the monitor
        match monitor.capture_image() {
            Ok(img) => {
                // xcap returns RGBA image; get raw bytes
                let raw: &[u8] = img.as_raw();
                if let Err(e) = stdin.write_all(raw) {
                    log::warn!("FFmpeg stdin write error: {}", e);
                    break;
                }
            }
            Err(e) => {
                log::warn!("Screen capture error: {}", e);
            }
        }
    }

    // Close stdin → tells FFmpeg to finalize the file
    drop(stdin);
    let _ = child.wait();
    log::info!("Screen capture finished");
    Ok(())
}
