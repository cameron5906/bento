import React, { useState, useEffect } from "react";
import type { StatusResponse } from "../types/supervisor";
import bentoLogo from "../assets/bento-logo.png";

const DEFAULT_MESSAGES = [
  "Unpacking your bento box...",
  "Warming up the containers...",
  "Preparing something delicious...",
  "Almost ready to serve...",
  "Arranging the compartments...",
  "Fresh ingredients loading...",
  "Plating your app...",
  "Adding the finishing touches...",
  "Your app is being boxed up...",
  "Seasoning the services...",
];

interface Props {
  status: StatusResponse;
  splashLogo?: string;
  splashMessages?: string[];
}

export function LoadingScreen({ status, splashLogo, splashMessages }: Props) {
  const percentage = Math.round(status.progress * 100);
  const messages = splashMessages && splashMessages.length > 0 ? splashMessages : DEFAULT_MESSAGES;
  const [msgIndex, setMsgIndex] = useState(() => Math.floor(Math.random() * messages.length));

  useEffect(() => {
    const interval = setInterval(() => {
      setMsgIndex((prev) => {
        let next;
        do { next = Math.floor(Math.random() * messages.length); } while (next === prev && messages.length > 1);
        return next;
      });
    }, 3000);
    return () => clearInterval(interval);
  }, [messages.length]);

  return (
    <div style={styles.container}>
      <div style={styles.content}>
        <img
          src={splashLogo || bentoLogo}
          alt="Loading"
          style={styles.logo}
        />
        <h2 style={styles.status}>{status.message}</h2>
        <p style={styles.splash}>{messages[msgIndex]}</p>
        <div style={styles.progressTrack}>
          <div
            style={{
              ...styles.progressFill,
              width: `${percentage}%`,
            }}
          />
        </div>
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
    maxWidth: 420,
    padding: 40,
  },
  logo: {
    width: 140,
    height: 140,
    objectFit: "contain",
    marginBottom: 24,
    filter: "drop-shadow(0 4px 12px rgba(99, 102, 241, 0.3))",
  },
  status: {
    fontSize: 16,
    fontWeight: 500,
    color: "#e0e0e0",
    marginBottom: 8,
  },
  splash: {
    fontSize: 14,
    color: "#6366f1",
    marginBottom: 24,
    fontStyle: "italic",
    minHeight: 20,
    transition: "opacity 0.3s",
  },
  progressTrack: {
    height: 4,
    background: "#333",
    borderRadius: 2,
    overflow: "hidden",
  },
  progressFill: {
    height: "100%",
    background: "linear-gradient(90deg, #6366f1, #818cf8)",
    borderRadius: 2,
    transition: "width 0.3s ease",
  },
};
