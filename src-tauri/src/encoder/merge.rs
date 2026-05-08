use anyhow::{anyhow, Result};
use std::process::Command;

use crate::encoder::ffmpeg::ffmpeg_program;

/// Merges screen.mp4 + webcam.mp4 + audio.wav into a final output.mp4
/// using FFmpeg's overlay (picture-in-picture) filter.
///
/// corner: "br" (bottom-right), "bl" (bottom-left), "tr" (top-right), "tl" (top-left)
pub fn merge_recordings(
    screen_path: &str,
    webcam_path: Option<&str>,
    audio_path: Option<&str>,
    output_path: &str,
    corner: &str,
) -> Result<()> {
    let overlay_pos = match corner {
        "br" => "W-w-20:H-h-20",
        "bl" => "20:H-h-20",
        "tr" => "W-w-20:20",
        "tl" => "20:20",
        _ => "W-w-20:H-h-20",
    };

    match (webcam_path, audio_path) {
        // Screen + Webcam + Audio
        (Some(webcam), Some(audio)) => {
            let filter = format!(
                "[1:v]scale=320:240,format=yuva420p,\
                 geq=lum='p(X,Y)':a='if(lte(hypot(X-160,Y-120),120),255,0)'[webcam];\
                 [0:v][webcam]overlay={}[out]",
                overlay_pos
            );
            let status = Command::new(ffmpeg_program())
                .args([
                    "-y",
                    "-i",
                    screen_path,
                    "-i",
                    webcam,
                    "-i",
                    audio,
                    "-filter_complex",
                    &filter,
                    "-map",
                    "[out]",
                    "-map",
                    "2:a",
                    "-c:v",
                    "libx264",
                    "-pix_fmt",
                    "yuv420p",
                    "-preset",
                    "fast",
                    "-crf",
                    "22",
                    "-c:a",
                    "aac",
                    "-b:a",
                    "192k",
                    output_path,
                ])
                .status()?;
            if !status.success() {
                return Err(anyhow!("FFmpeg merge (screen+webcam+audio) failed"));
            }
        }

        // Screen + Audio only
        (None, Some(audio)) => {
            let status = Command::new(ffmpeg_program())
                .args([
                    "-y",
                    "-i",
                    screen_path,
                    "-i",
                    audio,
                    "-c:v",
                    "copy",
                    "-c:a",
                    "aac",
                    "-b:a",
                    "192k",
                    output_path,
                ])
                .status()?;
            if !status.success() {
                return Err(anyhow!("FFmpeg merge (screen+audio) failed"));
            }
        }

        // Screen + Webcam, no audio
        (Some(webcam), None) => {
            let filter = format!(
                "[1:v]scale=320:240,format=yuva420p,\
                 geq=lum='p(X,Y)':a='if(lte(hypot(X-160,Y-120),120),255,0)'[webcam];\
                 [0:v][webcam]overlay={}[out]",
                overlay_pos
            );
            let status = Command::new(ffmpeg_program())
                .args([
                    "-y",
                    "-i",
                    screen_path,
                    "-i",
                    webcam,
                    "-filter_complex",
                    &filter,
                    "-map",
                    "[out]",
                    "-c:v",
                    "libx264",
                    "-pix_fmt",
                    "yuv420p",
                    "-preset",
                    "fast",
                    "-crf",
                    "22",
                    output_path,
                ])
                .status()?;
            if !status.success() {
                return Err(anyhow!("FFmpeg merge (screen+webcam) failed"));
            }
        }

        // Screen only — just rename/copy
        (None, None) => {
            std::fs::rename(screen_path, output_path)
                .or_else(|_| std::fs::copy(screen_path, output_path).map(|_| ()))?;
        }
    }

    log::info!("Merge complete → {}", output_path);
    Ok(())
}

/// Generate a thumbnail from the first second of a video using FFmpeg
pub fn generate_thumbnail(video_path: &str, thumb_path: &str) -> Result<()> {
    let status = Command::new(ffmpeg_program())
        .args([
            "-y",
            "-i",
            video_path,
            "-ss",
            "00:00:00.000",
            "-vframes",
            "1",
            "-vf",
            "scale=400:-1",
            thumb_path,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;

    if !status.success() {
        return Err(anyhow!("Thumbnail generation failed for {}", video_path));
    }
    Ok(())
}
