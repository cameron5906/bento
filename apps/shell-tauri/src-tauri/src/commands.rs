use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use tokio::sync::Mutex;

use craterun_core::types::StatusResponse;

use crate::supervisor_client::SupervisorClient;

pub struct SupervisorState {
    pub client: Mutex<Option<Arc<SupervisorClient>>>,
}

#[derive(Serialize)]
pub struct ConnectResponse {
    pub connected: bool,
    pub port: u16,
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
