import React from "react";
import type { StatusResponse } from "../types/supervisor";

interface Props {
  status: StatusResponse;
}

export function LoadingScreen({ status }: Props) {
  const percentage = Math.round(status.progress * 100);

  return (
    <div style={styles.container}>
      <div style={styles.content}>
        <div style={styles.spinner} />
        <h2 style={styles.message}>{status.message}</h2>
        <div style={styles.progressTrack}>
          <div
            style={{
              ...styles.progressFill,
              width: `${percentage}%`,
            }}
          />
        </div>
        {status.state.status === "checkingSystem" && (
          <p style={styles.hint}>This only happens the first time.</p>
        )}
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    height: "100vh",
    background: "linear-gradient(135deg, #0f0f0f 0%, #1a1a2e 100%)",
  },
  content: {
    textAlign: "center",
    maxWidth: 400,
    padding: 40,
  },
  spinner: {
    width: 48,
    height: 48,
    border: "3px solid #333",
    borderTopColor: "#6366f1",
    borderRadius: "50%",
    margin: "0 auto 24px",
    animation: "spin 1s linear infinite",
  },
  message: {
    fontSize: 18,
    fontWeight: 500,
    color: "#e0e0e0",
    marginBottom: 24,
  },
  progressTrack: {
    height: 4,
    background: "#333",
    borderRadius: 2,
    overflow: "hidden",
  },
  progressFill: {
    height: "100%",
    background: "#6366f1",
    borderRadius: 2,
    transition: "width 0.3s ease",
  },
  hint: {
    fontSize: 13,
    color: "#888",
    marginTop: 16,
  },
};
