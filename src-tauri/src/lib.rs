mod state;
mod capture;
mod encoder;
mod commands;

use commands::recording::{
    get_monitors, get_webcams,
    start_recording, stop_recording, get_recording_status,
    list_recordings, delete_recording, open_output_dir,
};
use state::RecordingState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(RecordingState::default())
        .invoke_handler(tauri::generate_handler![
            get_monitors,
            get_webcams,
            start_recording,
            stop_recording,
            get_recording_status,
            list_recordings,
            delete_recording,
            open_output_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running koom");
}
