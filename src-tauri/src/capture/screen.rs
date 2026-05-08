use anyhow::{anyhow, Result};
use std::io::Write;
#[cfg(not(target_os = "windows"))]
use std::process::Child;
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
#[cfg(not(target_os = "windows"))]
use std::time::Instant;
use xcap::Monitor;

/// Spawns an FFmpeg process that reads raw RGBA frames from stdin
/// and writes an H.264 MP4 at the given path.
#[cfg(not(target_os = "windows"))]
pub fn spawn_ffmpeg_screen(width: u32, height: u32, fps: u32, output_path: &str) -> Result<Child> {
    let child = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "rawvideo",
            "-pixel_format",
            "rgba",
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

pub fn run_screen_capture(
    monitor_index: usize,
    fps: u32,
    output_path: String,
    stop_signal: Arc<AtomicBool>,
) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let monitors = Monitor::all().map_err(|e| anyhow!("Failed to list monitors: {}", e))?;
        let monitor = monitors
            .get(monitor_index)
            .ok_or_else(|| anyhow!("Monitor index {} was not found", monitor_index))?;
        let x = monitor
            .x()
            .map_err(|e| anyhow!("Failed to read monitor x: {}", e))?;
        let y = monitor
            .y()
            .map_err(|e| anyhow!("Failed to read monitor y: {}", e))?;
        let width = monitor
            .width()
            .map_err(|e| anyhow!("Failed to read monitor width: {}", e))?;
        let height = monitor
            .height()
            .map_err(|e| anyhow!("Failed to read monitor height: {}", e))?;

        log::info!(
            "Screen capture: monitor {} at {},{} {}x{} @ {}fps -> {}",
            monitor_index,
            x,
            y,
            width,
            height,
            fps,
            output_path
        );

        let mut child = Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "gdigrab",
                "-framerate",
                &fps.to_string(),
                "-offset_x",
                &x.to_string(),
                "-offset_y",
                &y.to_string(),
                "-video_size",
                &format!("{}x{}", width, height),
                "-i",
                "desktop",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-preset",
                "ultrafast",
                "-tune",
                "zerolatency",
                "-crf",
                "25",
                &output_path,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Could not get FFmpeg stdin for screen capture"))?;

        // Monitor stop signal
        while !stop_signal.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(100));
        }

        // Send 'q' to FFmpeg to stop gracefully
        let _ = stdin.write_all(b"q");
        let status = child.wait()?;
        if !status.success() {
            return Err(anyhow!("FFmpeg screen capture failed"));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let monitors = Monitor::all().map_err(|e| anyhow!("Failed to list monitors: {}", e))?;
        let monitor = monitors
            .get(monitor_index)
            .ok_or_else(|| anyhow!("Monitor index {} was not found", monitor_index))?;
        let width = monitor
            .width()
            .map_err(|e| anyhow!("Failed to read monitor width: {}", e))?;
        let height = monitor
            .height()
            .map_err(|e| anyhow!("Failed to read monitor height: {}", e))?;
        let mut child = spawn_ffmpeg_screen(width, height, fps, &output_path)?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Could not get FFmpeg stdin for screen capture"))?;
        let frame_duration = Duration::from_secs_f64(1.0 / fps as f64);
        let mut next_frame = Instant::now();

        while !stop_signal.load(Ordering::Relaxed) {
            let now = Instant::now();
            if now < next_frame {
                std::thread::sleep(next_frame - now);
            }
            next_frame += frame_duration;

            let frame = monitor
                .capture_image()
                .map_err(|e| anyhow!("Failed to capture screen frame: {}", e))?;
            stdin.write_all(frame.as_raw())?;
        }

        drop(stdin);
        let status = child.wait()?;
        if !status.success() {
            return Err(anyhow!("FFmpeg screen capture failed"));
        }
    }

    log::info!("Screen capture finished");
    Ok(())
}
