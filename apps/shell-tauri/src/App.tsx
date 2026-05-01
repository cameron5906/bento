import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import bentoLogoImg from "./assets/bento-logo.png";
import { useSupervisorStatus } from "./hooks/useSupervisorStatus";
import { getScreen } from "./types/supervisor";
import { LoadingScreen } from "./screens/LoadingScreen";
import { ReadyScreen } from "./screens/ReadyScreen";
import { ErrorScreen } from "./screens/ErrorScreen";
import { BlockedScreen } from "./screens/BlockedScreen";

export default function App() {
  const { status, connected, error, connect, sendCommand } =
    useSupervisorStatus();
  const [launching, setLaunching] = useState(false);
  const [launchError, setLaunchError] = useState<string | null>(null);
  const [splashLogo, setSplashLogo] = useState<string | undefined>();
  const [splashMessages, setSplashMessages] = useState<string[]>([]);

  // Auto-launch supervisor on mount
  useEffect(() => {
    autoLaunch();
  }, []);

  async function autoLaunch() {
    setLaunching(true);
    try {
      const result = await invoke<{
        connected: boolean;
        port: number;
        splashLogo: string | null;
        splashMessages: string[];
      }>("launch_supervisor");

      if (result.splashLogo) setSplashLogo(result.splashLogo);
      if (result.splashMessages?.length) setSplashMessages(result.splashMessages);

      if (result.connected) {
        await connect(result.port, "auto");
      } else {
        setLaunchError("Supervisor started but not responding");
      }
    } catch (e) {
      // Auto-launch failed — fall back to dev connect screen
      setLaunchError(String(e));
    } finally {
      setLaunching(false);
    }
  }

  if (launching) {
    return <StartupScreen />;
  }

  // If auto-launch failed and we're not connected, show dev fallback
  if (!connected) {
    return (
      <DevConnectScreen
        onConnect={connect}
        error={launchError || error}
        onRetryLaunch={autoLaunch}
      />
    );
  }

  if (!status) {
    return <StartupScreen />;
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
      return <LoadingScreen status={status} splashLogo={splashLogo} splashMessages={splashMessages} />;
  }
}

function StartupScreen() {
  return (
    <div style={styles.center}>
      <img src={bentoLogoImg} alt="" style={{ width: 100, height: 100, objectFit: "contain", marginBottom: 16, opacity: 0.8 }} />
      <p style={styles.text}>Starting up...</p>
    </div>
  );
}

function DevConnectScreen({
  onConnect,
  error,
  onRetryLaunch,
}: {
  onConnect: (port: number, token: string) => void;
  error: string | null;
  onRetryLaunch: () => void;
}) {
  const [port, setPort] = useState("");
  const [token, setToken] = useState("");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const portNum = parseInt(port, 10);
    if (portNum && token) {
      onConnect(portNum, token);
    }
  };

  return (
    <div style={styles.center}>
      <div style={{ textAlign: "center", maxWidth: 380, padding: 40 }}>
        <h2 style={{ fontSize: 20, color: "#e0e0e0", marginBottom: 8 }}>
          Development Mode
        </h2>
        <p style={{ fontSize: 13, color: "#888", marginBottom: 16, lineHeight: 1.5 }}>
          Auto-launch failed. Connect to a running supervisor manually,
          or retry.
        </p>
        {error && (
          <p style={{ fontSize: 12, color: "#ef4444", marginBottom: 16 }}>
            {error}
          </p>
        )}
        <button onClick={onRetryLaunch} style={styles.button}>
          Retry Auto-Launch
        </button>
        <hr style={{ border: "none", borderTop: "1px solid #333", margin: "20px 0" }} />
        <form onSubmit={handleSubmit} style={{ display: "flex", flexDirection: "column", gap: 10 }}>
          <input
            type="number" placeholder="Port" value={port}
            onChange={(e) => setPort(e.target.value)}
            style={styles.input}
          />
          <input
            type="text" placeholder="Token" value={token}
            onChange={(e) => setToken(e.target.value)}
            style={styles.input}
          />
          <button type="submit" style={styles.button}>Connect</button>
        </form>
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  center: {
    display: "flex", alignItems: "center", justifyContent: "center",
    height: "100vh", background: "#0f0f0f", flexDirection: "column",
  },
  spinner: {
    width: 36, height: 36, border: "3px solid #333",
    borderTopColor: "#6366f1", borderRadius: "50%",
    marginBottom: 16, animation: "spin 1s linear infinite",
  },
  text: { color: "#888", fontSize: 14 },
  button: {
    padding: "10px 24px", background: "#6366f1", color: "#fff",
    border: "none", borderRadius: 6, fontSize: 14, cursor: "pointer",
  },
  input: {
    padding: "10px 14px", background: "#1a1a1a", border: "1px solid #333",
    borderRadius: 6, color: "#e0e0e0", fontSize: 14, outline: "none",
  },
};
