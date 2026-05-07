use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Overall recording state shared across Tauri commands
#[derive(Debug, Clone, PartialEq)]
pub enum RecordingStatus {
    Idle,
    Recording,
    Stopped,
}

pub struct RecordingState {
    pub status: RwLock<RecordingStatus>,
    /// Abort signal: when set to true, capture loops should stop
    pub stop_signal: Arc<std::sync::atomic::AtomicBool>,
    /// Output directory for recordings
    pub output_dir: RwLock<String>,
    /// Current session base name (timestamp-derived)
    pub session_name: RwLock<Option<String>>,
    /// Handle to the screen-capture + FFmpeg task
    pub screen_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// Handle to the webcam-capture + FFmpeg task
    pub webcam_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// Handle to the audio-capture task
    pub audio_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// Whether mic is enabled for this session
    pub mic_enabled: RwLock<bool>,
    /// Whether webcam is enabled for this session
    pub webcam_enabled: RwLock<bool>,
    /// Selected monitor index
    pub monitor_index: RwLock<usize>,
    /// Selected webcam index
    pub webcam_index: RwLock<usize>,
    /// Selected webcam corner: "br" | "bl" | "tr" | "tl"
    pub webcam_corner: RwLock<String>,
}

impl Default for RecordingState {
    fn default() -> Self {
        let video_dir = dirs::video_dir().unwrap_or_else(|| std::env::temp_dir());
        let default_output = video_dir.join("Koom").to_string_lossy().to_string();
        std::fs::create_dir_all(&default_output).ok();

        Self {
            status: RwLock::new(RecordingStatus::Idle),
            stop_signal: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            output_dir: RwLock::new(default_output),
            session_name: RwLock::new(None),
            screen_task: Mutex::new(None),
            webcam_task: Mutex::new(None),
            audio_task: Mutex::new(None),
            mic_enabled: RwLock::new(true),
            webcam_enabled: RwLock::new(false),
            monitor_index: RwLock::new(0),
            webcam_index: RwLock::new(0),
            webcam_corner: RwLock::new("br".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_recording_state_default() {
        let state = RecordingState::default();
        
        let status = state.status.read().await;
        assert_eq!(*status, RecordingStatus::Idle);
        
        let mic_enabled = state.mic_enabled.read().await;
        assert_eq!(*mic_enabled, true);
        
        let webcam_enabled = state.webcam_enabled.read().await;
        assert_eq!(*webcam_enabled, false);
        
        let corner = state.webcam_corner.read().await;
        assert_eq!(*corner, "br");
    }
}
