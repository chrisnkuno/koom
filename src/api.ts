import { invoke } from "@tauri-apps/api/core";

export interface MonitorInfo {
  index: number;
  name: string;
  width: number;
  height: number;
  is_primary: boolean;
}

export interface WebcamInfo {
  index: number;
  name: string;
}

export interface RecordingInfo {
  id: string;
  filename: string;
  path: string;
  thumbnail: string | null;
  duration_secs: number | null;
  created_at: string;
  size_bytes: number;
}

export interface RecordingConfig {
  monitor_index: number;
  mic_enabled: boolean;
  webcam_enabled: boolean;
  webcam_index: number;
  webcam_corner: "br" | "bl" | "tr" | "tl";
  fps: number;
}

export const koomApi = {
  getMonitors: (): Promise<MonitorInfo[]> => invoke("get_monitors"),
  getWebcams: (): Promise<WebcamInfo[]> => invoke("get_webcams"),
  startRecording: (config: RecordingConfig): Promise<string> =>
    invoke("start_recording", { config }),
  stopRecording: (): Promise<string> => invoke("stop_recording"),
  getRecordingStatus: (): Promise<"idle" | "recording" | "stopped"> =>
    invoke("get_recording_status"),
  listRecordings: (): Promise<RecordingInfo[]> => invoke("list_recordings"),
  deleteRecording: (path: string): Promise<void> =>
    invoke("delete_recording", { path }),
  openOutputDir: (): Promise<void> => invoke("open_output_dir"),
};
