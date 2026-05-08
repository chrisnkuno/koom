use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime},
};
use tauri::State;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::capture::audio::run_audio_capture;
use crate::capture::screen::run_screen_capture;
use crate::capture::webcam::run_webcam_capture;
use crate::encoder::ffmpeg::ensure_ffmpeg_available;
use crate::encoder::merge::{generate_thumbnail, merge_recordings};
use crate::state::RecordingState;

const MIN_VALID_FILE_SIZE: u64 = 1024;
const TASK_STOP_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecordingStatus {
    Idle,
    Starting,
    Recording,
    Stopping,
    Merging,
    Error(String),
}

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
    pub webcam_corner: String,
    pub fps: u32,
}

#[derive(Debug, Clone)]
pub struct RecordingSession {
    pub id: String,
    pub temp_dir: PathBuf,
    pub screen_file: PathBuf,
    pub webcam_file: PathBuf,
    pub audio_file: PathBuf,
    pub output_file: PathBuf,
    pub started_at: Instant,
    pub created_at: DateTime<Local>,
    pub config: RecordingConfig,
}

#[derive(Default)]
pub struct RecordingTasks {
    pub screen: Option<JoinHandle<()>>,
    pub webcam: Option<JoinHandle<()>>,
    pub audio: Option<JoinHandle<()>>,
}

#[derive(thiserror::Error, Debug)]
pub enum RecordingError {
    #[error("Already recording")]
    AlreadyRecording,

    #[error("No active recording")]
    NoActiveRecording,

    #[error("Invalid recording file: {0}")]
    InvalidRecording(String),

    #[error("Task timeout")]
    TaskTimeout,

    #[error("Merge failed: {0}")]
    Merge(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Capture error: {0}")]
    Capture(String),
}

impl From<std::io::Error> for RecordingError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

type Result<T> = std::result::Result<T, RecordingError>;

fn validate_file(path: &Path) -> bool {
    path.exists()
        && std::fs::metadata(path)
            .map(|m| m.len() > MIN_VALID_FILE_SIZE)
            .unwrap_or(false)
}

async fn spawn_capture_task<F>(name: &'static str, task: F) -> Result<JoinHandle<()>>
where
    F: FnOnce() -> anyhow::Result<()> + Send + 'static,
{
    Ok(tokio::spawn(async move {
        let result = tokio::task::spawn_blocking(task).await;

        match result {
            Ok(Ok(())) => {
                log::info!("{} task completed", name);
            }

            Ok(Err(e)) => {
                log::error!("{} task failed: {}", name, e);
            }

            Err(e) => {
                log::error!("{} task panicked: {}", name, e);
            }
        }
    }))
}

async fn wait_for_task(handle: JoinHandle<()>) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(TASK_STOP_TIMEOUT_SECS), handle)
        .await
        .map_err(|_| RecordingError::TaskTimeout)?
        .map_err(|e| RecordingError::Capture(e.to_string()))?;

    Ok(())
}

async fn cleanup_session(session: &RecordingSession) {
    let _ = std::fs::remove_file(&session.screen_file);
    let _ = std::fs::remove_file(&session.webcam_file);
    let _ = std::fs::remove_file(&session.audio_file);
    let _ = std::fs::remove_dir_all(&session.temp_dir);
}

fn finalize_recording(session: &RecordingSession) -> Result<()> {
    let screen_exists = validate_file(&session.screen_file);

    if !screen_exists {
        return Err(RecordingError::InvalidRecording(
            "Screen recording missing".into(),
        ));
    }

    let webcam = if session.config.webcam_enabled && validate_file(&session.webcam_file) {
        Some(session.webcam_file.to_string_lossy().to_string())
    } else {
        None
    };

    let audio = if session.config.mic_enabled && validate_file(&session.audio_file) {
        Some(session.audio_file.to_string_lossy().to_string())
    } else {
        None
    };

    let output = session.output_file.to_string_lossy().to_string();

    let screen = session.screen_file.to_string_lossy().to_string();

    merge_recordings(
        &screen,
        webcam.as_deref(),
        audio.as_deref(),
        &output,
        &session.config.webcam_corner,
    )
    .map_err(|e| RecordingError::Merge(e.to_string()))?;

    let thumb_path = output.replace(".mp4", "_thumb.jpg");

    if let Err(e) = generate_thumbnail(&output, &thumb_path) {
        log::warn!("Thumbnail generation failed: {}", e);
    }

    Ok(())
}

#[tauri::command]
pub async fn get_monitors() -> std::result::Result<Vec<MonitorInfo>, String> {
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

#[tauri::command]
pub async fn get_webcams() -> std::result::Result<Vec<WebcamInfo>, String> {
    let cameras = nokhwa::query(nokhwa::utils::ApiBackend::Auto).map_err(|e| e.to_string())?;

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

#[tauri::command]
pub async fn start_recording(
    config: RecordingConfig,
    state: State<'_, RecordingState>,
) -> std::result::Result<String, String> {
    {
        let status = state.status.read().await;

        if *status != RecordingStatus::Idle {
            return Err(RecordingError::AlreadyRecording.to_string());
        }
    }

    ensure_ffmpeg_available().map_err(|e| e.to_string())?;

    *state.status.write().await = RecordingStatus::Starting;

    let session_id = Uuid::new_v4().to_string();

    let temp_dir = std::env::temp_dir().join("koom").join(&session_id);

    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    let output_dir = state.output_dir.read().await.clone();

    let session = RecordingSession {
        id: session_id.clone(),
        temp_dir: temp_dir.clone(),

        screen_file: temp_dir.join("screen.mp4"),

        webcam_file: temp_dir.join("webcam.mp4"),

        audio_file: temp_dir.join("audio.wav"),

        output_file: Path::new(&output_dir).join(format!("{}.mp4", session_id)),

        started_at: Instant::now(),

        created_at: Local::now(),

        config: RecordingConfig {
            fps: config.fps.clamp(1, 60),
            ..config.clone()
        },
    };

    state.stop_signal.store(false, Ordering::SeqCst);

    {
        let mut current = state.current_session.write().await;
        *current = Some(session.clone());
    }

    let stop_signal: Arc<AtomicBool> = state.stop_signal.clone();

    let mut tasks = RecordingTasks::default();

    {
        let session_clone = session.clone();
        let stop = stop_signal.clone();

        tasks.screen = Some(
            spawn_capture_task("screen", move || {
                run_screen_capture(
                    session_clone.config.monitor_index,
                    session_clone.config.fps,
                    session_clone.screen_file.to_string_lossy().to_string(),
                    stop,
                )
            })
            .await
            .map_err(|e| e.to_string())?,
        );
    }

    if session.config.webcam_enabled {
        let session_clone = session.clone();
        let stop = stop_signal.clone();

        tasks.webcam = Some(
            spawn_capture_task("webcam", move || {
                run_webcam_capture(
                    session_clone.config.webcam_index,
                    session_clone.config.fps,
                    session_clone.webcam_file.to_string_lossy().to_string(),
                    stop,
                )
            })
            .await
            .map_err(|e| e.to_string())?,
        );
    }

    if session.config.mic_enabled {
        let session_clone = session.clone();
        let stop = stop_signal.clone();

        tasks.audio = Some(
            spawn_capture_task("audio", move || {
                run_audio_capture(session_clone.audio_file.to_string_lossy().to_string(), stop)
            })
            .await
            .map_err(|e| e.to_string())?,
        );
    }

    {
        let mut task_state = state.tasks.lock().await;
        *task_state = tasks;
    }

    *state.status.write().await = RecordingStatus::Recording;

    log::info!("Recording started: {}", session.id);

    Ok(session.id)
}

#[tauri::command]
pub async fn stop_recording(
    state: State<'_, RecordingState>,
) -> std::result::Result<String, String> {
    {
        let status = state.status.read().await;

        if *status != RecordingStatus::Recording {
            return Err(RecordingError::NoActiveRecording.to_string());
        }
    }

    *state.status.write().await = RecordingStatus::Stopping;

    state.stop_signal.store(true, Ordering::SeqCst);

    let mut tasks = state.tasks.lock().await;

    if let Some(handle) = tasks.screen.take() {
        wait_for_task(handle).await.map_err(|e| e.to_string())?;
    }

    if let Some(handle) = tasks.webcam.take() {
        wait_for_task(handle).await.map_err(|e| e.to_string())?;
    }

    if let Some(handle) = tasks.audio.take() {
        wait_for_task(handle).await.map_err(|e| e.to_string())?;
    }

    drop(tasks);

    *state.status.write().await = RecordingStatus::Merging;

    let session = { state.current_session.read().await.clone() }
        .ok_or_else(|| RecordingError::NoActiveRecording.to_string())?;

    let output = session.output_file.to_string_lossy().to_string();

    let finalize_result = tokio::task::spawn_blocking({
        let session = session.clone();

        move || finalize_recording(&session)
    })
    .await;

    match finalize_result {
        Ok(Ok(())) => {
            cleanup_session(&session).await;

            *state.status.write().await = RecordingStatus::Idle;

            *state.current_session.write().await = None;

            log::info!(
                "Recording saved: {} (started {}, duration {:?})",
                output,
                session.created_at.format("%Y-%m-%d %H:%M:%S"),
                session.started_at.elapsed()
            );

            Ok(output)
        }

        Ok(Err(e)) => {
            *state.status.write().await = RecordingStatus::Error(e.to_string());

            Err(e.to_string())
        }

        Err(e) => {
            let err = e.to_string();

            *state.status.write().await = RecordingStatus::Error(err.clone());

            Err(err)
        }
    }
}

#[tauri::command]
pub async fn get_recording_status(
    state: State<'_, RecordingState>,
) -> std::result::Result<String, String> {
    let status = state.status.read().await;

    Ok(match &*status {
        RecordingStatus::Idle => "idle".into(),
        RecordingStatus::Starting => "starting".into(),
        RecordingStatus::Recording => "recording".into(),
        RecordingStatus::Stopping => "stopping".into(),
        RecordingStatus::Merging => "merging".into(),
        RecordingStatus::Error(e) => {
            format!("error: {}", e)
        }
    })
}

#[tauri::command]
pub async fn list_recordings(
    state: State<'_, RecordingState>,
) -> std::result::Result<Vec<RecordingInfo>, String> {
    let output_dir = state.output_dir.read().await.clone();

    let dir = Path::new(&output_dir);

    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut recordings: Vec<(SystemTime, RecordingInfo)> = vec![];

    let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("mp4") {
            continue;
        }

        let meta = entry.metadata().map_err(|e| e.to_string())?;

        let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);

        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let id = filename.replace(".mp4", "");

        let thumb_path = path.to_string_lossy().replace(".mp4", "_thumb.jpg");

        let thumbnail = if Path::new(&thumb_path).exists() {
            Some(thumb_path)
        } else {
            None
        };

        let created_at: DateTime<Local> = modified.into();

        recordings.push((
            modified,
            RecordingInfo {
                id,
                filename,

                path: path.to_string_lossy().to_string(),

                thumbnail,

                duration_secs: None,

                created_at: created_at.format("%Y-%m-%dT%H:%M:%S").to_string(),

                size_bytes: meta.len(),
            },
        ));
    }

    recordings.sort_by(|a, b| b.0.cmp(&a.0));

    Ok(recordings.into_iter().map(|(_, r)| r).collect())
}

#[tauri::command]
pub async fn delete_recording(path: String) -> std::result::Result<(), String> {
    std::fs::remove_file(&path).map_err(|e| e.to_string())?;

    let thumb = path.replace(".mp4", "_thumb.jpg");

    let _ = std::fs::remove_file(thumb);

    Ok(())
}

#[tauri::command]
pub async fn open_output_dir(state: State<'_, RecordingState>) -> std::result::Result<(), String> {
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
