import React, { useState } from "react";
import type { UserFacingError } from "../types/supervisor";
import { DiagnosticsPanel } from "./DiagnosticsPanel";

interface Props {
  error: UserFacingError;
  onRetry: () => void;
  onRepair: () => void;
}

export function ErrorScreen({ error, onRetry, onRepair }: Props) {
  const [showDetails, setShowDetails] = useState(false);

  return (
    <div style={styles.container}>
      <div style={styles.content}>
        <div style={styles.icon}>!</div>
        <h2 style={styles.title}>{error.userTitle}</h2>
        <p style={styles.message}>{error.userMessage}</p>

        <div style={styles.actions}>
          {error.actions.includes("RETRY") && (
            <button style={styles.primaryButton} onClick={onRetry}>
              Try Again
            </button>
          )}
          {error.actions.includes("REPAIR") && (
            <button style={styles.secondaryButton} onClick={onRepair}>
              Repair
            </button>
          )}
          <button
            style={styles.textButton}
            onClick={() => setShowDetails(!showDetails)}
          >
            {showDetails ? "Hide Details" : "Show Details"}
          </button>
        </div>

        {showDetails && (
          <DiagnosticsPanel technicalMessage={error.technicalMessage} />
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
    maxWidth: 460,
    padding: 40,
  },
  icon: {
    width: 56,
    height: 56,
    borderRadius: "50%",
    background: "#ef4444",
    color: "#fff",
    fontSize: 28,
    fontWeight: 700,
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    margin: "0 auto 20px",
  },
  title: {
    fontSize: 20,
    fontWeight: 600,
    color: "#e0e0e0",
    marginBottom: 8,
  },
  message: {
    fontSize: 14,
    color: "#999",
    marginBottom: 28,
    lineHeight: 1.5,
  },
  actions: {
    display: "flex",
    gap: 12,
    justifyContent: "center",
    flexWrap: "wrap" as const,
  },
  primaryButton: {
    padding: "10px 24px",
    background: "#6366f1",
    color: "#fff",
    border: "none",
    borderRadius: 6,
    fontSize: 14,
    fontWeight: 500,
    cursor: "pointer",
  },
  secondaryButton: {
    padding: "10px 24px",
    background: "#333",
    color: "#e0e0e0",
    border: "1px solid #555",
    borderRadius: 6,
    fontSize: 14,
    fontWeight: 500,
    cursor: "pointer",
  },
  textButton: {
    padding: "10px 16px",
    background: "transparent",
    color: "#888",
    border: "none",
    fontSize: 13,
    cursor: "pointer",
    textDecoration: "underline",
  },
};
