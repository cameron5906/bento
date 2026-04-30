import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { StatusResponse } from "../types/supervisor";

const POLL_INTERVAL = 500;

export function useSupervisorStatus() {
  const [status, setStatus] = useState<StatusResponse | null>(null);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const connect = useCallback(async (port: number, token: string) => {
    try {
      await invoke("connect_supervisor", { port, token });
      setConnected(true);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const sendCommand = useCallback(async (command: string) => {
    try {
      await invoke("send_command", { command });
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    if (!connected) return;

    const poll = async () => {
      try {
        const result = await invoke<StatusResponse>("get_status");
        setStatus(result);
        setError(null);
      } catch (e) {
        setError(String(e));
      }
    };

    poll();
    intervalRef.current = setInterval(poll, POLL_INTERVAL);

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, [connected]);

  return { status, connected, error, connect, sendCommand };
}
