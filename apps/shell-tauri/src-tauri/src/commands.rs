use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use tokio::sync::Mutex;

use bento_core::types::{StatusResponse, SupervisorSockInfo};

use crate::supervisor_client::SupervisorClient;

pub struct SupervisorState {
    pub client: Mutex<Option<Arc<SupervisorClient>>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectResponse {
    pub connected: bool,
    pub port: u16,
    pub splash_logo: Option<String>,
    pub splash_messages: Vec<String>,
}

/// Launch or reconnect to the supervisor. If an existing supervisor is
/// already running (sock file exists and API responds), reuse it instead
/// of spawning a new one. This makes reopening the app instant.
#[tauri::command]
pub async fn launch_supervisor(
    state: State<'_, SupervisorState>,
) -> Result<ConnectResponse, String> {
    let exe_dir = std::env::current_exe()
        .map(|p| p.parent().unwrap_or_else(|| std::path::Path::new(".")).to_path_buf())
        .unwrap_or_else(|_| PathBuf::from("."));

    let bundle_path = exe_dir.join("bundle");

    let manifest_path = bundle_path.join("manifest.json");
    let (app_id, app_name) = if manifest_path.exists() {
        let content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("failed to read manifest: {}", e))?;
        let manifest: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("failed to parse manifest: {}", e))?;
        (
            manifest["app"]["id"].as_str().unwrap_or("unknown").to_string(),
            manifest["app"]["name"].as_str().unwrap_or("App").to_string(),
        )
    } else {
        ("unknown".to_string(), "App".to_string())
    };

    let sock_path = supervisor_sock_path(&app_id, &app_name);

    // Try to reconnect to an existing supervisor first
    if sock_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&sock_path) {
            if let Ok(info) = serde_json::from_str::<SupervisorSockInfo>(&content) {
                let client = Arc::new(SupervisorClient::new(info.port, info.token.clone()));
                if let Ok(status) = client.get_status().await {
                    // Existing supervisor is alive — reuse it
                    *state.client.lock().await = Some(client);

                    // Send prepare if it hasn't started yet (progress == 0)
                    if status.progress == 0.0 && status.app_url.is_none() {
                        if let Some(ref c) = *state.client.lock().await {
                            let _ = c.post_command("prepare").await;
                        }
                    }

                    let splash = read_splash_config(&bundle_path);
                    return Ok(ConnectResponse {
                        connected: true,
                        port: info.port,
                        splash_logo: splash.0,
                        splash_messages: splash.1,
                    });
                }
            }
        }
        // Sock file exists but supervisor is dead — clean up
        let _ = std::fs::remove_file(&sock_path);
    }

    // No existing supervisor — spawn a new one
    let supervisor_path = find_supervisor(&exe_dir);

    let log_dir = sock_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_file = std::fs::File::create(log_dir.join("supervisor.log"))
        .map_err(|e| format!("failed to create log file: {}", e))?;
    let log_err = log_file.try_clone()
        .map_err(|e| format!("failed to clone log file: {}", e))?;

    let mut cmd = tokio::process::Command::new(&supervisor_path);
    cmd.arg(bundle_path.to_string_lossy().as_ref())
        .stdout(log_file)
        .stderr(log_err);

    // Hide console window on Windows
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x00000008); // DETACHED_PROCESS
    }

    // Don't kill_on_drop — the supervisor stays alive after the shell closes,
    // with a 15-minute idle timeout before stopping containers
    let _child = cmd
        .spawn()
        .map_err(|e| format!("failed to launch supervisor: {}", e))?;

    // Wait for supervisor to write its sock file
    let sock_info = wait_for_sock_file(&sock_path).await?;

    let client = Arc::new(SupervisorClient::new(sock_info.port, sock_info.token.clone()));

    // Wait for API to respond
    for _ in 0..20 {
        if client.get_status().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }

    *state.client.lock().await = Some(client);

    // Kick off the state machine
    if let Some(ref c) = *state.client.lock().await {
        let _ = c.post_command("prepare").await;
    }

    let splash = read_splash_config(&bundle_path);
    Ok(ConnectResponse {
        connected: true,
        port: sock_info.port,
        splash_logo: splash.0,
        splash_messages: splash.1,
    })
}

#[tauri::command]
pub async fn connect_supervisor(
    state: State<'_, SupervisorState>,
    port: u16,
    token: String,
) -> Result<ConnectResponse, String> {
    let client = Arc::new(SupervisorClient::new(port, token));
    let status = client.get_status().await;
    let connected = status.is_ok();
    *state.client.lock().await = Some(client);
    Ok(ConnectResponse { connected, port, splash_logo: None, splash_messages: Vec::new() })
}

#[tauri::command]
pub async fn get_status(
    state: State<'_, SupervisorState>,
) -> Result<StatusResponse, String> {
    let client = {
        let guard = state.client.lock().await;
        guard
            .as_ref()
            .ok_or_else(|| "supervisor not connected".to_string())?
            .clone()
    };
    client.get_status().await
}

#[tauri::command]
pub async fn send_command(
    state: State<'_, SupervisorState>,
    command: String,
) -> Result<(), String> {
    let client = {
        let guard = state.client.lock().await;
        guard
            .as_ref()
            .ok_or_else(|| "supervisor not connected".to_string())?
            .clone()
    };
    client.post_command(&command).await
}

fn find_supervisor(exe_dir: &std::path::Path) -> PathBuf {
    #[cfg(windows)]
    let suffixes = ["-supervisor.exe", "bento-supervisor.exe"];
    #[cfg(not(windows))]
    let suffixes = ["-supervisor", "bento-supervisor"];

    for entry in std::fs::read_dir(exe_dir).into_iter().flatten() {
        if let Ok(e) = entry {
            let name = e.file_name().to_string_lossy().to_string();
            if suffixes.iter().any(|s| name.ends_with(s)) {
                return e.path();
            }
        }
    }

    #[cfg(windows)]
    return exe_dir.join("bento-supervisor.exe");
    #[cfg(not(windows))]
    return exe_dir.join("bento-supervisor");
}

fn supervisor_sock_path(app_id: &str, app_name: &str) -> PathBuf {
    let id = bento_core::AppId::new(app_id)
        .unwrap_or_else(|_| bento_core::AppId::new("com.bento.unknown").unwrap());
    let paths = bento_core::paths::AppPaths::new(id, app_name.to_string());
    paths.supervisor_sock_file()
}

async fn wait_for_sock_file(path: &std::path::Path) -> Result<SupervisorSockInfo, String> {
    for _ in 0..40 {
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| format!("failed to read sock file: {}", e))?;
            let info: SupervisorSockInfo = serde_json::from_str(&content)
                .map_err(|e| format!("failed to parse sock file: {}", e))?;
            return Ok(info);
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    Err(format!(
        "supervisor did not start within 10 seconds (expected {})",
        path.display()
    ))
}

/// Read splash config from bundle/shell/shell-config.json
fn read_splash_config(bundle_path: &std::path::Path) -> (Option<String>, Vec<String>) {
    let config_path = bundle_path.join("shell").join("shell-config.json");
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
            let logo = config["splash"]["logo"].as_str().map(|s| s.to_string());
            let messages: Vec<String> = config["splash"]["messages"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            return (logo, messages);
        }
    }
    (None, Vec::new())
}
