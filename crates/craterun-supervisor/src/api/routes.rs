use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use craterun_core::types::StatusResponse;
use craterun_core::SupervisorState;

use crate::diagnostics::DiagnosticsBundle;

use super::AppState;
use crate::engine::SupervisorCommand;

pub async fn get_status(State(state): State<AppState>) -> Json<StatusResponse> {
    let current = state.state_rx.borrow().clone();

    let (app_url, error) = match &current {
        SupervisorState::Ready { app_url } => (Some(app_url.clone()), None),
        SupervisorState::FailedRecoverable { error } => (None, Some(error.clone())),
        SupervisorState::FailedBlocked { error } => (None, Some(error.clone())),
        _ => (None, None),
    };

    let response = StatusResponse {
        app_id: state.manifest.app.id.clone(),
        state: current.clone(),
        message: current.user_message(&state.manifest.app.name).to_string(),
        progress: current.progress(),
        app_url,
        error,
        services: Vec::new(),
    };

    Json(response)
}

pub async fn post_prepare(State(state): State<AppState>) -> StatusCode {
    send_command(&state, SupervisorCommand::Prepare).await
}

pub async fn post_start(State(state): State<AppState>) -> StatusCode {
    send_command(&state, SupervisorCommand::Start).await
}

pub async fn post_stop(State(state): State<AppState>) -> StatusCode {
    send_command(&state, SupervisorCommand::Stop).await
}

pub async fn post_restart(State(state): State<AppState>) -> StatusCode {
    send_command(&state, SupervisorCommand::Restart).await
}

pub async fn post_repair(State(state): State<AppState>) -> StatusCode {
    send_command(&state, SupervisorCommand::Repair).await
}

#[derive(Deserialize)]
pub struct ResetDataBody {
    #[serde(default)]
    confirm: bool,
}

pub async fn post_reset_data(
    State(state): State<AppState>,
    Json(body): Json<ResetDataBody>,
) -> StatusCode {
    send_command(
        &state,
        SupervisorCommand::ResetData {
            confirm: body.confirm,
        },
    )
    .await
}

pub async fn get_logs() -> Json<Vec<craterun_core::types::LogLine>> {
    // Full implementation would query the runtime adapter for service logs.
    // For now, return empty — the diagnostics export captures logs separately.
    Json(Vec::new())
}

pub async fn get_diagnostics(State(state): State<AppState>) -> Json<DiagnosticsBundle> {
    let current = state.state_rx.borrow().clone();

    let bundle = DiagnosticsBundle::collect(
        &state.manifest,
        &current,
        &state.adapter_name,
        Vec::new(), // logs would be populated from runtime adapter
    );

    Json(bundle)
}

async fn send_command(state: &AppState, cmd: SupervisorCommand) -> StatusCode {
    match state.cmd_tx.send(cmd).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
