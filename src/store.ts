import { create } from "zustand";
import { koomApi, MonitorInfo, WebcamInfo, RecordingInfo, RecordingConfig } from "./api";

type AppStatus = "idle" | "recording" | "stopped";
type Corner = "br" | "bl" | "tr" | "tl";
type ActiveView = "recorder" | "library" | "settings";

interface RecordingStore {
  // Status
  status: AppStatus;
  sessionName: string | null;
  elapsedSecs: number;
  timerRef: ReturnType<typeof setInterval> | null;

  // Devices
  monitors: MonitorInfo[];
  webcams: WebcamInfo[];
  selectedMonitor: number;
  selectedWebcam: number;

  // Config
  micEnabled: boolean;
  webcamEnabled: boolean;
  webcamCorner: Corner;
  fps: number;

  // Library
  recordings: RecordingInfo[];
  activeView: ActiveView;

  // Error
  error: string | null;

  // Actions
  loadDevices: () => Promise<void>;
  loadRecordings: () => Promise<void>;
  startRecording: () => Promise<void>;
  stopRecording: () => Promise<void>;
  deleteRecording: (path: string) => Promise<void>;
  openOutputDir: () => Promise<void>;
  setMonitor: (i: number) => void;
  setWebcam: (i: number) => void;
  setMicEnabled: (v: boolean) => void;
  setWebcamEnabled: (v: boolean) => void;
  setWebcamCorner: (c: Corner) => void;
  setFps: (f: number) => void;
  setActiveView: (v: ActiveView) => void;
  clearError: () => void;
}

export const useRecordingStore = create<RecordingStore>((set, get) => ({
  status: "idle",
  sessionName: null,
  elapsedSecs: 0,
  timerRef: null,

  monitors: [],
  webcams: [],
  selectedMonitor: 0,
  selectedWebcam: 0,

  micEnabled: true,
  webcamEnabled: false,
  webcamCorner: "br",
  fps: 30,

  recordings: [],
  activeView: "recorder",

  error: null,

  loadDevices: async () => {
    try {
      const [monitors, webcams] = await Promise.all([
        koomApi.getMonitors(),
        koomApi.getWebcams().catch(() => [] as WebcamInfo[]),
      ]);
      set({ monitors, webcams });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  loadRecordings: async () => {
    try {
      const recordings = await koomApi.listRecordings();
      set({ recordings });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  startRecording: async () => {
    const s = get();
    if (s.status === "recording") return;
    set({ error: null });
    try {
      const config: RecordingConfig = {
        monitor_index: s.selectedMonitor,
        mic_enabled: s.micEnabled,
        webcam_enabled: s.webcamEnabled,
        webcam_index: s.selectedWebcam,
        webcam_corner: s.webcamCorner,
        fps: s.fps,
      };
      const name = await koomApi.startRecording(config);
      const timerRef = setInterval(() => {
        set((st) => ({ elapsedSecs: st.elapsedSecs + 1 }));
      }, 1000);
      set({ status: "recording", sessionName: name, elapsedSecs: 0, timerRef });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  stopRecording: async () => {
    const s = get();
    if (s.status !== "recording") return;
    // Stop the timer
    if (s.timerRef) clearInterval(s.timerRef);
    set({ status: "stopped", timerRef: null });
    try {
      await koomApi.stopRecording();
      set({ status: "idle", sessionName: null });
      await get().loadRecordings();
    } catch (e) {
      set({ error: String(e), status: "idle" });
    }
  },

  deleteRecording: async (path: string) => {
    try {
      await koomApi.deleteRecording(path);
      await get().loadRecordings();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  openOutputDir: async () => {
    try {
      await koomApi.openOutputDir();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  setMonitor: (i) => set({ selectedMonitor: i }),
  setWebcam: (i) => set({ selectedWebcam: i }),
  setMicEnabled: (v) => set({ micEnabled: v }),
  setWebcamEnabled: (v) => set({ webcamEnabled: v }),
  setWebcamCorner: (c) => set({ webcamCorner: c }),
  setFps: (f) => set({ fps: f }),
  setActiveView: (v) => set({ activeView: v }),
  clearError: () => set({ error: null }),
}));
