import { useEffect, useRef, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { useRecordingStore } from "../store";

function formatTime(secs: number): string {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;
  if (h > 0) {
    return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  }
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}



// ── Corner Picker ────────────────────────────────────────────────────────── //
type Corner = "br" | "bl" | "tr" | "tl";
const CORNERS: { id: Corner; label: string }[] = [
  { id: "tl", label: "↖" },
  { id: "tr", label: "↗" },
  { id: "bl", label: "↙" },
  { id: "br", label: "↘" },
];

function CornerPicker() {
  const { webcamCorner, setWebcamCorner } = useRecordingStore();
  return (
    <div className="corner-picker">
      {CORNERS.map(({ id, label }) => (
        <button
          key={id}
          id={`corner-${id}`}
          className={`corner-btn ${webcamCorner === id ? "active" : ""}`}
          onClick={() => setWebcamCorner(id)}
          title={`Webcam position: ${id}`}
        >
          {label}
        </button>
      ))}
    </div>
  );
}

// ── Webcam Preview ───────────────────────────────────────────────────────── //
function WebcamPreview({ webcamEnabled }: { webcamEnabled: boolean }) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const [hasWebcam, setHasWebcam] = useState(false);

  useEffect(() => {
    if (!webcamEnabled) {
      if (videoRef.current?.srcObject) {
        const tracks = (videoRef.current.srcObject as MediaStream).getTracks();
        tracks.forEach((t) => t.stop());
        videoRef.current.srcObject = null;
      }
      setHasWebcam(false);
      return;
    }
    navigator.mediaDevices
      .getUserMedia({ video: true })
      .then((stream) => {
        if (videoRef.current) {
          videoRef.current.srcObject = stream;
          setHasWebcam(true);
        }
      })
      .catch(() => setHasWebcam(false));
    return () => {
      if (videoRef.current?.srcObject) {
        const tracks = (videoRef.current.srcObject as MediaStream).getTracks();
        tracks.forEach((t) => t.stop());
      }
    };
  }, [webcamEnabled]);

  if (!webcamEnabled) return null;

  return (
    <div className="webcam-preview">
      {hasWebcam ? (
        <video ref={videoRef} autoPlay muted playsInline />
      ) : (
        <div className="no-cam">No camera</div>
      )}
    </div>
  );
}

// ── Main Recorder View ───────────────────────────────────────────────────── //
export function RecorderView() {
  const {
    status,
    elapsedSecs,
    sessionName,
    monitors,
    webcams,
    selectedMonitor,
    selectedWebcam,
    micEnabled,
    webcamEnabled,
    fps,
    startRecording,
    stopRecording,
    setMonitor,
    setWebcam,
    setMicEnabled,
    setWebcamEnabled,
    setFps,
  } = useRecordingStore();

  const isRecording = status === "recording";
  const isProcessing = status === "stopped";

  const handleToggleRecording = async () => {
    if (isProcessing) return;
    if (isRecording) {
      await stopRecording();
    } else {
      await startRecording();
    }
  };

  return (
    <div className="recorder-view">
      {/* Header */}
      <div className="view-header">
        <div>
          <h1 className="view-title">New Recording</h1>
          <p className="view-subtitle">Configure and start your recording session</p>
        </div>
        {webcamEnabled && <WebcamPreview webcamEnabled={webcamEnabled} />}
      </div>

      {/* Record Controls */}
      <motion.div
        className={`record-controls ${isRecording ? "recording" : ""}`}
        animate={{
          borderColor: isRecording
            ? "rgba(239, 68, 68, 0.4)"
            : "rgba(255, 255, 255, 0.06)",
        }}
        transition={{ duration: 0.3 }}
      >
        {/* Main Button */}
        <button
          id="btn-record-toggle"
          className={`record-btn ${isRecording ? "stop" : ""} ${isProcessing ? "processing" : ""}`}
          onClick={handleToggleRecording}
          disabled={isProcessing}
          title={isRecording ? "Stop recording" : "Start recording"}
        >
          {isProcessing ? (
            "⟳"
          ) : isRecording ? (
            <svg width="22" height="22" viewBox="0 0 24 24" fill="white">
              <rect x="6" y="6" width="12" height="12" rx="2" />
            </svg>
          ) : (
            <svg width="18" height="18" viewBox="0 0 24 24" fill="white">
              <circle cx="12" cy="12" r="6" />
              <circle cx="12" cy="12" r="10" fill="none" stroke="white" strokeWidth="2" />
            </svg>
          )}
        </button>

        {/* Info */}
        <div className="record-info">
          <AnimatePresence mode="wait">
            <motion.span
              key={status}
              className={`record-status-label ${status}`}
              initial={{ opacity: 0, y: -6 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 6 }}
              transition={{ duration: 0.15 }}
            >
              {isProcessing
                ? "Processing…"
                : isRecording
                ? "● Recording"
                : "Ready"}
            </motion.span>
          </AnimatePresence>

          <div className="record-timer">{formatTime(elapsedSecs)}</div>
          {sessionName && (
            <div className="record-session">{sessionName}</div>
          )}
        </div>

        {/* Indicators */}
        <div className="indicators">
          <div className={`indicator ${micEnabled && isRecording ? "active" : ""}`}>
            <div className="indicator-dot" />
            Mic
          </div>
          <div className={`indicator ${webcamEnabled && isRecording ? "active" : ""}`}>
            <div className="indicator-dot" />
            Cam
          </div>
          <div className={`indicator ${isRecording ? "active" : ""}`}>
            <div className="indicator-dot" />
            Screen
          </div>
        </div>
      </motion.div>

      {/* Source Config */}
      <div className="card">
        <div className="card-title">Capture Source</div>
        <div className="config-grid">
          <div className="config-field">
            <label className="config-label" htmlFor="monitor-select">
              Monitor
            </label>
            <select
              id="monitor-select"
              className="config-select"
              value={selectedMonitor}
              onChange={(e) => setMonitor(Number(e.target.value))}
              disabled={isRecording}
            >
              {monitors.length === 0 ? (
                <option value={0}>Loading monitors…</option>
              ) : (
                monitors.map((m) => (
                  <option key={m.index} value={m.index}>
                    {m.name} ({m.width}×{m.height}){m.is_primary ? " ★" : ""}
                  </option>
                ))
              )}
            </select>
          </div>

          <div className="config-field">
            <label className="config-label">Frame Rate</label>
            <div className="fps-slider">
              <input
                id="fps-slider"
                type="range"
                min={10}
                max={60}
                step={5}
                value={fps}
                onChange={(e) => setFps(Number(e.target.value))}
                disabled={isRecording}
              />
              <span className="fps-value">{fps} fps</span>
            </div>
          </div>
        </div>
      </div>

      {/* Audio + Webcam Config */}
      <div className="card">
        <div className="card-title">Inputs</div>

        <div className="toggle-row">
          <div className="toggle-info">
            <div className="toggle-label">Microphone</div>
            <div className="toggle-desc">Record system default mic</div>
          </div>
          <button
            id="toggle-mic"
            className={`toggle ${micEnabled ? "on" : ""}`}
            onClick={() => setMicEnabled(!micEnabled)}
            disabled={isRecording}
          />
        </div>

        <div className="toggle-row">
          <div className="toggle-info">
            <div className="toggle-label">Webcam Overlay</div>
            <div className="toggle-desc">
              {webcams.length === 0
                ? "No webcam detected"
                : `${webcams[selectedWebcam]?.name ?? "Camera"} — circular PiP`}
            </div>
          </div>
          <button
            id="toggle-webcam"
            className={`toggle ${webcamEnabled ? "on" : ""}`}
            onClick={() => setWebcamEnabled(!webcamEnabled)}
            disabled={isRecording || webcams.length === 0}
          />
        </div>

        {webcamEnabled && (
          <div className="toggle-row" style={{ flexWrap: "wrap", gap: "12px" }}>
            <div className="config-field" style={{ flex: 1, minWidth: "160px" }}>
              <label className="config-label" htmlFor="webcam-select">
                Camera
              </label>
              <select
                id="webcam-select"
                className="config-select"
                value={selectedWebcam}
                onChange={(e) => setWebcam(Number(e.target.value))}
                disabled={isRecording}
              >
                {webcams.map((w) => (
                  <option key={w.index} value={w.index}>
                    {w.name}
                  </option>
                ))}
              </select>
            </div>

            <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
              <span className="config-label">Corner</span>
              <CornerPicker />
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
