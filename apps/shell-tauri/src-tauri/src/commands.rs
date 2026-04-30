use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use tokio::sync::Mutex;

use craterun_core::types::{StatusResponse, SupervisorSockInfo};

use crate::supervisor_client::SupervisorClient;

pub struct SupervisorState {
    pub client: Mutex<Option<Arc<SupervisorClient>>>,
    pub child: Mutex<Option<tokio::process::Child>>,
}

#[derive(Serialize)]
pub struct ConnectResponse {
    pub connected: bool,
    pub port: u16,
}

/// Spawn the supervisor binary as a child process, wait for its sock file,
/// and auto-connect. This is the production startup path — the consumer
/// never sees connection details.
#[tauri::command]
pub async fn launch_supervisor(
    state: State<'_, SupervisorState>,
) -> Result<ConnectResponse, String> {
    // Locate the supervisor binary next to the shell executable
    let exe_dir = std::env::current_exe()
        .map(|p| p.parent().unwrap_or_else(|| std::path::Path::new(".")).to_path_buf())
        .unwrap_or_else(|_| PathBuf::from("."));

    let supervisor_path = find_supervisor(&exe_dir);
    let bundle_path = exe_dir.join("bundle");

    // Read app ID first so we can clean up stale state before launching
    let manifest_path = bundle_path.join("manifest.json");
    let app_id = if manifest_path.exists() {
        let content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("failed to read manifest: {}", e))?;
        let manifest: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("failed to parse manifest: {}", e))?;
        manifest["app"]["id"]
            .as_str()
            .unwrap_or("unknown")
            .to_string()
    } else {
        "unknown".to_string()
    };

    // Delete stale sock file so we only read the fresh one from the new supervisor
    let sock_path = supervisor_sock_path(&app_id);
    let _ = std::fs::remove_file(&sock_path);

    let child = tokio::process::Command::new(&supervisor_path)
        .arg(bundle_path.to_string_lossy().as_ref())
        .kill_on_drop(true) // stop supervisor when shell exits
        .spawn()
        .map_err(|e| format!("failed to launch supervisor: {}", e))?;

    *state.child.lock().await = Some(child);

    // Wait for the new supervisor to write its sock file
    let sock_info = wait_for_sock_file(&sock_path).await?;

    let client = Arc::new(SupervisorClient::new(sock_info.port, sock_info.token.clone()));

    // Wait for the API to respond
    for _ in 0..20 {
        if client.get_status().await.is_ok() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }

    *state.client.lock().await = Some(client);

    // Auto-send Prepare to kick off the state machine
    if let Some(ref c) = *state.client.lock().await {
        let _ = c.post_command("prepare").await;
    }

    Ok(ConnectResponse {
        connected: true,
        port: sock_info.port,
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
    Ok(ConnectResponse { connected, port })
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
    // Look for any *-supervisor.exe or craterun-supervisor.exe next to the shell
    for entry in std::fs::read_dir(exe_dir).into_iter().flatten() {
        if let Ok(e) = entry {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with("-supervisor.exe") || name == "craterun-supervisor.exe" {
                return e.path();
            }
        }
    }
    exe_dir.join("craterun-supervisor.exe")
}

fn supervisor_sock_path(app_id: &str) -> PathBuf {
    let local_app_data = std::env::var("LOCALAPPDATA")
        .unwrap_or_else(|_| "C:\\Users\\Default\\AppData\\Local".into());
    PathBuf::from(local_app_data)
        .join("CrateRun")
        .join("Apps")
        .join(app_id)
        .join("config")
        .join("supervisor.sock.json")
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
