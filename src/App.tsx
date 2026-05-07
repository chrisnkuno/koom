import { useEffect } from "react";
import { useRecordingStore } from "./store";
import { RecorderView } from "./components/RecorderView";
import { LibraryView } from "./components/LibraryView";
import "./index.css";

// SVG Icon components
const IconRecord = () => (
  <svg viewBox="0 0 24 24" fill="currentColor" className="nav-icon">
    <circle cx="12" cy="12" r="8" />
  </svg>
);

const IconLibrary = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="nav-icon">
    <rect x="3" y="3" width="7" height="7" rx="1" />
    <rect x="14" y="3" width="7" height="7" rx="1" />
    <rect x="3" y="14" width="7" height="7" rx="1" />
    <rect x="14" y="14" width="7" height="7" rx="1" />
  </svg>
);

export default function App() {
  const { activeView, setActiveView, loadDevices, loadRecordings, error, clearError } =
    useRecordingStore();

  useEffect(() => {
    loadDevices();
    loadRecordings();
  }, []);

  // Auto-clear error after 5s
  useEffect(() => {
    if (!error) return;
    const t = setTimeout(clearError, 5000);
    return () => clearTimeout(t);
  }, [error]);

  return (
    <div className="app">
      {/* Sidebar */}
      <aside className="sidebar">
        <div className="sidebar-logo">
          <div className="logo-dot" />
          Koom
        </div>

        <button
          id="nav-recorder"
          className={`nav-item ${activeView === "recorder" ? "active" : ""}`}
          onClick={() => setActiveView("recorder")}
        >
          <IconRecord />
          Recorder
        </button>

        <button
          id="nav-library"
          className={`nav-item ${activeView === "library" ? "active" : ""}`}
          onClick={() => {
            setActiveView("library");
            loadRecordings();
          }}
        >
          <IconLibrary />
          Library
        </button>
      </aside>

      {/* Main */}
      <main className="main-content">
        {activeView === "recorder" && <RecorderView />}
        {activeView === "library" && <LibraryView />}
      </main>

      {/* Error Toast */}
      {error && (
        <div className="error-toast" onClick={clearError}>
          <span>⚠</span>
          <span>{error}</span>
          <span style={{ marginLeft: "auto", opacity: 0.6, fontSize: "11px" }}>click to dismiss</span>
        </div>
      )}
    </div>
  );
}
