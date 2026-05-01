mod routes;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use tokio::sync::{mpsc, watch};

use bento_core::SupervisorState;

use bento_bundle::manifest::compiled_manifest::CompiledManifest;

use crate::engine::SupervisorCommand;

#[derive(Clone)]
pub struct AppState {
    pub state_rx: watch::Receiver<SupervisorState>,
    pub cmd_tx: mpsc::Sender<SupervisorCommand>,
    pub manifest: CompiledManifest,
    pub adapter_name: String,
    pub token: String,
    pub active: Arc<AtomicBool>,
}

pub fn router(state: AppState, active: Arc<AtomicBool>) -> Router {
    let token = state.token.clone();

    Router::new()
        .route("/status", get(routes::get_status))
        .route("/prepare", post(routes::post_prepare))
        .route("/start", post(routes::post_start))
        .route("/stop", post(routes::post_stop))
        .route("/restart", post(routes::post_restart))
        .route("/repair", post(routes::post_repair))
        .route("/reset-data", post(routes::post_reset_data))
        .route("/logs", get(routes::get_logs))
        .route("/diagnostics/export", get(routes::get_diagnostics))
        .layer(middleware::from_fn(move |req, next| {
            let t = token.clone();
            let a = active.clone();
            activity_middleware(t, a, req, next)
        }))
        .with_state(state)
}

async fn activity_middleware(
    token: String,
    active: Arc<AtomicBool>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) if header == format!("Bearer {}", token) => {
            // Mark activity on every authenticated request — resets idle timer
            active.store(true, Ordering::Relaxed);
            Ok(next.run(req).await)
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
