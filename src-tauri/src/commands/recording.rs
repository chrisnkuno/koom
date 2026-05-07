use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use tauri::State;
use chrono::Local;

use crate::state::{RecordingState, RecordingStatus};
use crate::capture::screen::run_screen_capture;
use crate::capture::webcam::run_webcam_capture;
use crate::capture::audio::run_audio_capture;
use crate::encoder::merge::{merge_recordings, generate_thumbnail};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonitorInfo {
    pub index: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebcamInfo {
    pub index: usize,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecordingInfo {
    pub id: String,
    pub filename: String,
    pub path: String,
    pub thumbnail: Option<String>,
    pub duration_secs: Option<f64>,
    pub created_at: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecordingConfig {
    pub monitor_index: usize,
    pub mic_enabled: bool,
    pub webcam_enabled: bool,
    pub webcam_index: usize,
    pub webcam_corner: String, // "br" | "bl" | "tr" | "tl"
    pub fps: u32,
}

/// List all available monitors
#[tauri::command]
pub async fn get_monitors() -> Result<Vec<MonitorInfo>, String> {
    let monitors = xcap::Monitor::all().map_err(|e| e.to_string())?;
    let infos = monitors
        .into_iter()
        .enumerate()
        .map(|(i, m)| MonitorInfo {
            index: i,
            name: m.name().unwrap_or_else(|_| "Unknown".to_string()),
            width: m.width().unwrap_or(1920),
            height: m.height().unwrap_or(1080),
            is_primary: m.is_primary().unwrap_or(false),
        })
        .collect();
    Ok(infos)
}

/// List all available webcams
#[tauri::command]
pub async fn get_webcams() -> Result<Vec<WebcamInfo>, String> {
    let cameras = nokhwa::query(nokhwa::utils::ApiBackend::Auto)
        .map_err(|e| e.to_string())?;
    let infos = cameras
        .into_iter()
        .enumerate()
        .map(|(i, info)| WebcamInfo {
            index: i,
            name: info.human_name().to_string(),
        })
        .collect();
    Ok(infos)
}

/// Start a recording session
#[tauri::command]
pub async fn start_recording(
    config: RecordingConfig,
    state: State<'_, RecordingState>,
) -> Result<String, String> {
    // Prevent double-start
    {
        let status = state.status.read().await;
        if *status == RecordingStatus::Recording {
            return Err("Already recording".to_string());
        }
    }

    // Reset stop signal
    state.stop_signal.store(false, Ordering::Relaxed);

    // Generate session name
    let session_name = Local::now().format("koom_%Y%m%d_%H%M%S").to_string();
    {
        let mut sn = state.session_name.write().await;
        *sn = Some(session_name.clone());
    }

    // Update config
    {
        let mut mi = state.monitor_index.write().await;
        *mi = config.monitor_index;
        let mut me = state.mic_enabled.write().await;
        *me = config.mic_enabled;
        let mut we = state.webcam_enabled.write().await;
        *we = config.webcam_enabled;
        let mut wi = state.webcam_index.write().await;
        *wi = config.webcam_index;
        let mut wc = state.webcam_corner.write().await;
        *wc = config.webcam_corner.clone();
    }

    let temp_dir = std::env::temp_dir();
    let tmp_screen = temp_dir.join(format!("{}_screen.mp4", session_name)).to_string_lossy().to_string();
    let tmp_webcam = temp_dir.join(format!("{}_webcam.mp4", session_name)).to_string_lossy().to_string();
    let tmp_audio = temp_dir.join(format!("{}_audio.wav", session_name)).to_string_lossy().to_string();

    let fps = config.fps.max(1).min(60);

    // ── Screen capture task ─────────────────────────────────────────────────
    let stop_screen = state.stop_signal.clone();
    let screen_path = tmp_screen.clone();
    let monitor_idx = config.monitor_index;
    let screen_handle = tokio::spawn(async move {
        tokio::task::spawn_blocking(move || {
            if let Err(e) = run_screen_capture(monitor_idx, fps, screen_path, stop_screen) {
                log::error!("Screen capture error: {}", e);
            }
        })
        .await
        .ok();
    });
    *state.screen_task.lock().await = Some(screen_handle);

    // ── Webcam capture task ─────────────────────────────────────────────────
    if config.webcam_enabled {
        let stop_webcam = state.stop_signal.clone();
        let webcam_path = tmp_webcam.clone();
        let cam_idx = config.webcam_index;
        let webcam_handle = tokio::spawn(async move {
            tokio::task::spawn_blocking(move || {
                if let Err(e) = run_webcam_capture(cam_idx, fps, webcam_path, stop_webcam) {
                    log::error!("Webcam capture error: {}", e);
                }
            })
            .await
            .ok();
        });
        *state.webcam_task.lock().await = Some(webcam_handle);
    }

    // ── Audio capture task (blocking thread) ───────────────────────────────
    if config.mic_enabled {
        let stop_audio = state.stop_signal.clone();
        let audio_path = tmp_audio.clone();
        let audio_handle = tokio::spawn(async move {
            tokio::task::spawn_blocking(move || {
                if let Err(e) = run_audio_capture(audio_path, stop_audio) {
                    log::error!("Audio capture error: {}", e);
                }
            })
            .await
            .ok();
        });
        *state.audio_task.lock().await = Some(audio_handle);
    }

    // Mark as recording
    *state.status.write().await = RecordingStatus::Recording;

    log::info!("Recording started: {}", session_name);
    Ok(session_name)
}

/// Stop the current recording session and trigger FFmpeg merge
#[tauri::command]
pub async fn stop_recording(state: State<'_, RecordingState>) -> Result<String, String> {
    {
        let status = state.status.read().await;
        if *status != RecordingStatus::Recording {
            return Err("Not recording".to_string());
        }
    }

    // Signal all loops to stop
    state.stop_signal.store(true, Ordering::Relaxed);

    // Wait for all tasks to complete
    if let Some(handle) = state.screen_task.lock().await.take() {
        handle.await.ok();
    }
    if let Some(handle) = state.webcam_task.lock().await.take() {
        handle.await.ok();
    }
    if let Some(handle) = state.audio_task.lock().await.take() {
        handle.await.ok();
    }

    *state.status.write().await = RecordingStatus::Stopped;

    // Retrieve session info
    let session_name = state.session_name.read().await.clone().unwrap_or_default();
    let output_dir = state.output_dir.read().await.clone();
    let mic_enabled = *state.mic_enabled.read().await;
    let webcam_enabled = *state.webcam_enabled.read().await;
    let corner = state.webcam_corner.read().await.clone();

    let temp_dir = std::env::temp_dir();
    let tmp_screen = temp_dir.join(format!("{}_screen.mp4", session_name)).to_string_lossy().to_string();
    let tmp_webcam = temp_dir.join(format!("{}_webcam.mp4", session_name)).to_string_lossy().to_string();
    let tmp_audio = temp_dir.join(format!("{}_audio.wav", session_name)).to_string_lossy().to_string();
    let final_output = std::path::Path::new(&output_dir)
        .join(format!("{}.mp4", session_name))
        .to_string_lossy()
        .to_string();

    // Run merge in a blocking thread
    let final_out_clone = final_output.clone();
    tokio::task::spawn_blocking(move || {
        let webcam = if webcam_enabled && std::fs::metadata(&tmp_webcam).map(|m| m.len() > 1024).unwrap_or(false) {
            Some(tmp_webcam.as_str())
        } else {
            None
        };
        let audio = if mic_enabled && std::fs::metadata(&tmp_audio).map(|m| m.len() > 1024).unwrap_or(false) {
            Some(tmp_audio.as_str())
        } else {
            None
        };

        merge_recordings(&tmp_screen, webcam, audio, &final_out_clone, &corner)
            .map_err(|e| log::error!("Merge error: {}", e))
            .ok();

        // Generate thumbnail
        let thumb_path = final_out_clone.replace(".mp4", "_thumb.jpg");
        generate_thumbnail(&final_out_clone, &thumb_path)
            .map_err(|e| log::warn!("Thumbnail error: {}", e))
            .ok();

        // Cleanup temp files
        for p in &[&tmp_screen, &tmp_audio] {
            std::fs::remove_file(p).ok();
        }
    })
    .await
    .map_err(|e| e.to_string())?;

    *state.status.write().await = RecordingStatus::Idle;
    *state.session_name.write().await = None;

    log::info!("Recording saved → {}", final_output);
    Ok(final_output)
}

/// Get current recording status
#[tauri::command]
pub async fn get_recording_status(state: State<'_, RecordingState>) -> Result<String, String> {
    let status = state.status.read().await;
    Ok(match *status {
        RecordingStatus::Idle => "idle".to_string(),
        RecordingStatus::Recording => "recording".to_string(),
        RecordingStatus::Stopped => "stopped".to_string(),
    })
}

/// List all recordings in the output directory
#[tauri::command]
pub async fn list_recordings(state: State<'_, RecordingState>) -> Result<Vec<RecordingInfo>, String> {
    let output_dir = state.output_dir.read().await.clone();
    let dir = std::path::Path::new(&output_dir);
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut recordings = vec![];
    let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("mp4") {
            continue;
        }
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let id = filename.replace(".mp4", "");
        let thumb_path = path.to_string_lossy().replace(".mp4", "_thumb.jpg");
        let thumbnail = if std::path::Path::new(&thumb_path).exists() {
            Some(thumb_path)
        } else {
            None
        };

        let created_at = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| {
                let dt = chrono::DateTime::<Local>::from(std::time::SystemTime::UNIX_EPOCH + d);
                dt.format("%Y-%m-%dT%H:%M:%S").to_string()
            })
            .unwrap_or_default();

        recordings.push(RecordingInfo {
            id,
            filename,
            path: path.to_string_lossy().to_string(),
            thumbnail,
            duration_secs: None, // TODO: probe with ffprobe
            created_at,
            size_bytes: meta.len(),
        });
    }

    // Sort newest first
    recordings.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(recordings)
}

/// Delete a recording by path
#[tauri::command]
pub async fn delete_recording(path: String) -> Result<(), String> {
    std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    let thumb = path.replace(".mp4", "_thumb.jpg");
    std::fs::remove_file(&thumb).ok();
    Ok(())
}

/// Open the output directory in the system file manager
#[tauri::command]
pub async fn open_output_dir(state: State<'_, RecordingState>) -> Result<(), String> {
    let dir = state.output_dir.read().await.clone();
    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer")
        .arg(&dir)
        .spawn()
        .map_err(|e| e.to_string())?;

    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg(&dir)
        .spawn()
        .map_err(|e| e.to_string())?;

    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open")
        .arg(&dir)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
