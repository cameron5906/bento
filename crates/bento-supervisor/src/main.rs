mod api;
mod diagnostics;
mod engine;
mod health;
mod proxy;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::{mpsc, watch};
use tracing_subscriber::EnvFilter;

use bento_bundle::bundle::BundleReader;
use bento_core::types::SupervisorSockInfo;
use bento_core::{AppId, SupervisorState};
use bento_runtime::adapters::existing_docker::ExistingDockerAdapter;
use bento_runtime::RuntimeAdapter;

use engine::{SupervisorCommand, SupervisorEngine};

/// How long to keep containers running after the last client disconnects.
/// Generous timeout so reopening the app is instant.
const IDLE_TIMEOUT_SECS: u64 = 15 * 60; // 15 minutes

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let bundle_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./dist/bundle"));

    let reader = BundleReader::new(&bundle_path);
    let manifest = reader.read_manifest()?;
    let app_id = AppId::new(&manifest.app.id)
        .map_err(|e| anyhow::anyhow!("invalid app ID: {}", e))?;

    let adapter: Arc<dyn RuntimeAdapter> = Arc::new(ExistingDockerAdapter::new());

    let (cmd_tx, cmd_rx) = mpsc::channel::<SupervisorCommand>(32);
    let (state_tx, state_rx) = watch::channel(SupervisorState::InstalledNotPrepared);

    let api_port = allocate_port()?;
    let token = generate_token();

    let paths = bento_core::paths::AppPaths::new(app_id.clone(), manifest.app.name.clone());
    write_sock_info(&paths, api_port, &token)?;

    // Track client activity for the idle timer
    let active = Arc::new(AtomicBool::new(true));

    let api_state = api::AppState {
        state_rx,
        cmd_tx,
        manifest: manifest.clone(),
        adapter_name: adapter.adapter_name().to_string(),
        token: token.clone(),
        active: active.clone(),
    };

    let shutdown_adapter = adapter.clone();
    let shutdown_app_id = app_id.clone();
    let sock_file = paths.supervisor_sock_file();

    let engine = SupervisorEngine::new(
        app_id,
        manifest,
        adapter,
        reader,
        state_tx,
        cmd_rx,
    );

    let engine_handle = tokio::spawn(async move {
        engine.run().await;
    });

    tracing::info!("Supervisor API on 127.0.0.1:{}", api_port);

    let active_for_idle = active.clone();

    // Idle timer: if no API requests for IDLE_TIMEOUT_SECS, shut down
    let idle_handle = tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;

            if active_for_idle.swap(false, Ordering::Relaxed) {
                // Activity detected in the last 30s, reset
                continue;
            }

            // No activity — start the idle countdown
            tracing::info!("No client activity, starting idle countdown...");
            tokio::time::sleep(std::time::Duration::from_secs(IDLE_TIMEOUT_SECS)).await;

            if !active_for_idle.load(Ordering::Relaxed) {
                tracing::info!("Idle timeout reached — shutting down");
                break;
            }
        }
    });

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", api_port)).await?;
    let router = api::router(api_state, active);

    let server = axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            tokio::select! {
                _ = shutdown_signal() => {}
                _ = idle_handle => {}
            }
        });

    server
        .await
        .map_err(|e| anyhow::anyhow!("server error: {}", e))?;

    // Cleanup
    tracing::info!("Shutting down — stopping containers");
    let _ = shutdown_adapter.stop_app(&shutdown_app_id).await;
    let _ = std::fs::remove_file(&sock_file);

    engine_handle.abort();
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install ctrl+c handler");
    }
    tracing::info!("Received shutdown signal");
}

fn allocate_port() -> anyhow::Result<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let bytes: [u8; 32] = rng.random();
    format!(
        "bt_tok_{}",
        bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>()
    )
}

fn write_sock_info(
    paths: &bento_core::paths::AppPaths,
    port: u16,
    token: &str,
) -> anyhow::Result<()> {
    let config_dir = paths.config_dir();
    std::fs::create_dir_all(&config_dir)?;
    let info = SupervisorSockInfo {
        port,
        token: token.to_string(),
    };
    let json = serde_json::to_string_pretty(&info)?;
    std::fs::write(paths.supervisor_sock_file(), json)?;
    Ok(())
}
