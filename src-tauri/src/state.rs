use std::sync::{atomic::AtomicBool, Arc};
use tokio::sync::{Mutex, RwLock};

use crate::commands::recording::{RecordingSession, RecordingStatus, RecordingTasks};

pub struct RecordingState {
    pub status: RwLock<RecordingStatus>,
    pub stop_signal: Arc<AtomicBool>,
    pub output_dir: RwLock<String>,
    pub current_session: RwLock<Option<RecordingSession>>,
    pub tasks: Mutex<RecordingTasks>,
}

impl Default for RecordingState {
    fn default() -> Self {
        let video_dir = dirs::video_dir().unwrap_or_else(std::env::temp_dir);
        let default_output = video_dir.join("Koom").to_string_lossy().to_string();
        std::fs::create_dir_all(&default_output).ok();

        Self {
            status: RwLock::new(RecordingStatus::Idle),
            stop_signal: Arc::new(AtomicBool::new(false)),
            output_dir: RwLock::new(default_output),
            current_session: RwLock::new(None),
            tasks: Mutex::new(RecordingTasks::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_recording_state_default() {
        let state = RecordingState::default();

        assert_eq!(*state.status.read().await, RecordingStatus::Idle);
        assert!(state.current_session.read().await.is_none());
        assert!(!state.stop_signal.load(std::sync::atomic::Ordering::Relaxed));
    }
}
