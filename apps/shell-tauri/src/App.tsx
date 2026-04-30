import React from "react";
import { useSupervisorStatus } from "./hooks/useSupervisorStatus";
import { getScreen } from "./types/supervisor";
import { LoadingScreen } from "./screens/LoadingScreen";
import { ReadyScreen } from "./screens/ReadyScreen";
import { ErrorScreen } from "./screens/ErrorScreen";
import { BlockedScreen } from "./screens/BlockedScreen";

export default function App() {
  const { status, connected, error, connect, sendCommand } =
    useSupervisorStatus();

  if (!connected) {
    return <ConnectScreen onConnect={connect} error={error} />;
  }

  if (!status) {
    return <LoadingPlaceholder />;
  }

  const screen = getScreen(status.state);

  switch (screen) {
    case "ready":
      return <ReadyScreen appUrl={status.appUrl!} />;

    case "error":
      return (
        <ErrorScreen
          error={status.error!}
          onRetry={() => sendCommand("restart")}
          onRepair={() => sendCommand("repair")}
        />
      );

    case "blocked":
      return <BlockedScreen error={status.error!} />;

    case "loading":
    default:
      return <LoadingScreen status={status} />;
  }
}

function ConnectScreen({
  onConnect,
  error,
}: {
  onConnect: (port: number, token: string) => void;
  error: string | null;
}) {
  const [port, setPort] = React.useState("");
  const [token, setToken] = React.useState("");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const portNum = parseInt(port, 10);
    if (portNum && token) {
      onConnect(portNum, token);
    }
  };

  return (
    <div style={styles.connectContainer}>
      <div style={styles.connectContent}>
        <h2 style={styles.connectTitle}>Connect to Supervisor</h2>
        <p style={styles.connectHint}>
          In production, the shell connects automatically. For development,
          enter the supervisor port and token.
        </p>
        <form onSubmit={handleSubmit} style={styles.form}>
          <input
            type="number"
            placeholder="Port"
            value={port}
            onChange={(e) => setPort(e.target.value)}
            style={styles.input}
          />
          <input
            type="text"
            placeholder="Token"
            value={token}
            onChange={(e) => setToken(e.target.value)}
            style={styles.input}
          />
          <button type="submit" style={styles.connectButton}>
            Connect
          </button>
        </form>
        {error && <p style={styles.connectError}>{error}</p>}
      </div>
    </div>
  );
}

function LoadingPlaceholder() {
  return (
    <div style={styles.connectContainer}>
      <p style={{ color: "#888" }}>Connecting to supervisor...</p>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  connectContainer: {
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    height: "100vh",
    background: "#0f0f0f",
  },
  connectContent: {
    textAlign: "center",
    maxWidth: 360,
    padding: 40,
  },
  connectTitle: {
    fontSize: 20,
    fontWeight: 600,
    color: "#e0e0e0",
    marginBottom: 8,
  },
  connectHint: {
    fontSize: 13,
    color: "#888",
    marginBottom: 24,
    lineHeight: 1.5,
  },
  form: {
    display: "flex",
    flexDirection: "column" as const,
    gap: 12,
  },
  input: {
    padding: "10px 14px",
    background: "#1a1a1a",
    border: "1px solid #333",
    borderRadius: 6,
    color: "#e0e0e0",
    fontSize: 14,
    outline: "none",
  },
  connectButton: {
    padding: "10px 24px",
    background: "#6366f1",
    color: "#fff",
    border: "none",
    borderRadius: 6,
    fontSize: 14,
    fontWeight: 500,
    cursor: "pointer",
  },
  connectError: {
    fontSize: 13,
    color: "#ef4444",
    marginTop: 12,
  },
};
