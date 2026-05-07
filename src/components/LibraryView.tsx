import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { useRecordingStore } from "../store";
import { RecordingInfo } from "../api";

function formatDate(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}

function formatBytes(bytes: number): string {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

const IconFolder = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
    <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
  </svg>
);

const IconPlay = () => (
  <svg viewBox="0 0 24 24" fill="white" width="20" height="20">
    <polygon points="5,3 19,12 5,21" />
  </svg>
);

const IconTrash = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="14" height="14">
    <polyline points="3 6 5 6 21 6" />
    <path d="M19 6l-1 14H6L5 6" />
    <path d="M10 11v6M14 11v6" />
  </svg>
);

const IconFilm = () => (
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="empty-icon">
    <rect x="2" y="2" width="20" height="20" rx="3" />
    <path d="M7 2v20M17 2v20M2 12h20M2 7h5M17 7h5M2 17h5M17 17h5" />
  </svg>
);

// Video player modal
function VideoModal({
  recording,
  onClose,
}: {
  recording: RecordingInfo;
  onClose: () => void;
}) {
  return (
    <motion.div
      className="modal-overlay"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      onClick={onClose}
    >
      <motion.div
        className="modal"
        initial={{ scale: 0.92, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        exit={{ scale: 0.92, opacity: 0 }}
        transition={{ type: "spring", damping: 24, stiffness: 320 }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <span className="modal-title">{recording.filename}</span>
          <button
            id="modal-close"
            className="btn btn-ghost"
            onClick={onClose}
            style={{ padding: "4px 10px", fontSize: "12px" }}
          >
            ✕ Close
          </button>
        </div>
        <video
          src={`asset://localhost/${encodeURIComponent(recording.path)}`}
          controls
          autoPlay
        />
      </motion.div>
    </motion.div>
  );
}

export function LibraryView() {
  const { recordings, deleteRecording, openOutputDir } = useRecordingStore();
  const [playing, setPlaying] = useState<RecordingInfo | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);

  return (
    <div className="library-view">
      {/* Header */}
      <div className="library-header">
        <div>
          <h1 className="view-title">Library</h1>
          <p className="view-subtitle">
            {recordings.length} recording{recordings.length !== 1 ? "s" : ""}
          </p>
        </div>
        <button
          id="btn-open-folder"
          className="btn btn-ghost"
          onClick={openOutputDir}
          style={{ gap: "8px" }}
        >
          <IconFolder />
          Open Folder
        </button>
      </div>

      {/* Grid */}
      {recordings.length === 0 ? (
        <div className="empty-state">
          <IconFilm />
          <div>
            <div style={{ fontSize: "16px", fontWeight: 600, color: "var(--text-secondary)", marginBottom: "8px" }}>
              No recordings yet
            </div>
            <div style={{ fontSize: "13px" }}>
              Go to Recorder and start your first session
            </div>
          </div>
        </div>
      ) : (
        <div className="library-grid">
          <AnimatePresence>
            {recordings.map((rec, i) => (
              <motion.div
                key={rec.id}
                className="recording-card"
                id={`recording-${rec.id}`}
                initial={{ opacity: 0, y: 16 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, scale: 0.95 }}
                transition={{ delay: i * 0.05 }}
                onClick={() => setPlaying(rec)}
              >
                {/* Thumbnail */}
                <div className="recording-thumb">
                  {rec.thumbnail ? (
                    <img
                      src={`asset://localhost/${encodeURIComponent(rec.thumbnail)}`}
                      alt={rec.filename}
                    />
                  ) : (
                    <div className="recording-thumb-placeholder">
                      <IconFilm />
                    </div>
                  )}
                  <div className="recording-play-overlay">
                    <div className="play-icon">
                      <IconPlay />
                    </div>
                  </div>
                </div>

                {/* Meta */}
                <div className="recording-meta">
                  <div className="recording-name">{rec.filename}</div>
                  <div className="recording-details">
                    <span>{formatDate(rec.created_at)}</span>
                    <span>{formatBytes(rec.size_bytes)}</span>
                  </div>
                </div>

                {/* Actions */}
                <div className="recording-actions">
                  {confirmDelete === rec.path ? (
                    <>
                      <button
                        id={`confirm-delete-${rec.id}`}
                        className="action-btn"
                        onClick={(e) => {
                          e.stopPropagation();
                          deleteRecording(rec.path);
                          setConfirmDelete(null);
                        }}
                        title="Confirm delete"
                        style={{ background: "rgba(239,68,68,0.8)", fontSize: "10px", width: "auto", padding: "0 8px" }}
                      >
                        Delete
                      </button>
                      <button
                        className="action-btn safe"
                        onClick={(e) => {
                          e.stopPropagation();
                          setConfirmDelete(null);
                        }}
                        title="Cancel"
                      >
                        ✕
                      </button>
                    </>
                  ) : (
                    <button
                      id={`delete-${rec.id}`}
                      className="action-btn"
                      onClick={(e) => {
                        e.stopPropagation();
                        setConfirmDelete(rec.path);
                      }}
                      title="Delete recording"
                    >
                      <IconTrash />
                    </button>
                  )}
                </div>
              </motion.div>
            ))}
          </AnimatePresence>
        </div>
      )}

      {/* Video modal */}
      <AnimatePresence>
        {playing && (
          <VideoModal recording={playing} onClose={() => setPlaying(null)} />
        )}
      </AnimatePresence>
    </div>
  );
}
