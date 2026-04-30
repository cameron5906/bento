use std::sync::Arc;

use tokio::sync::{mpsc, watch};

use craterun_bundle::bundle::BundleReader;
use craterun_bundle::manifest::compiled_manifest::CompiledManifest;
use craterun_core::error::{ErrorSeverity, UserFacingError};
use craterun_core::{AppId, CrateRunError, SupervisorState};
use craterun_runtime::types::{ImageRef, PlannedService, RemoveOptions, RuntimePlan, VolumeMount};
use craterun_runtime::RuntimeAdapter;

use crate::health::HealthChecker;
use crate::proxy::ReverseProxy;

#[derive(Debug)]
pub enum SupervisorCommand {
    Prepare,
    Start,
    Stop,
    Restart,
    Repair,
    ResetData { confirm: bool },
}

pub struct SupervisorEngine {
    app_id: AppId,
    manifest: CompiledManifest,
    adapter: Arc<dyn RuntimeAdapter>,
    reader: BundleReader,
    state_tx: watch::Sender<SupervisorState>,
    cmd_rx: mpsc::Receiver<SupervisorCommand>,
}

impl SupervisorEngine {
    pub fn new(
        app_id: AppId,
        manifest: CompiledManifest,
        adapter: Arc<dyn RuntimeAdapter>,
        reader: BundleReader,
        state_tx: watch::Sender<SupervisorState>,
        cmd_rx: mpsc::Receiver<SupervisorCommand>,
    ) -> Self {
        Self {
            app_id,
            manifest,
            adapter,
            reader,
            state_tx,
            cmd_rx,
        }
    }

    pub async fn run(mut self) {
        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                SupervisorCommand::Prepare => {
                    self.run_prepare_sequence().await;
                }
                SupervisorCommand::Start => {
                    self.run_start_sequence().await;
                }
                SupervisorCommand::Stop => {
                    self.set_state(SupervisorState::Stopping);
                    if let Err(e) = self.adapter.stop_app(&self.app_id).await {
                        tracing::error!("stop failed: {}", e);
                    }
                    self.set_state(SupervisorState::Stopped);
                }
                SupervisorCommand::Restart => {
                    self.set_state(SupervisorState::Stopping);
                    let _ = self.adapter.stop_app(&self.app_id).await;
                    self.set_state(SupervisorState::Stopped);
                    self.run_start_sequence().await;
                }
                SupervisorCommand::Repair => {
                    self.set_state(SupervisorState::Repairing);
                    let _ = self.adapter.stop_app(&self.app_id).await;
                    let _ = self
                        .adapter
                        .remove_app(
                            &self.app_id,
                            RemoveOptions {
                                remove_volumes: false,
                                remove_runtime: false,
                            },
                        )
                        .await;
                    self.run_prepare_sequence().await;
                }
                SupervisorCommand::ResetData { confirm } => {
                    if !confirm {
                        continue;
                    }
                    self.set_state(SupervisorState::Stopping);
                    let _ = self.adapter.stop_app(&self.app_id).await;
                    let _ = self
                        .adapter
                        .remove_app(
                            &self.app_id,
                            RemoveOptions {
                                remove_volumes: true,
                                remove_runtime: false,
                            },
                        )
                        .await;
                    self.set_state(SupervisorState::Stopped);
                }
            }
        }
    }

    async fn run_prepare_sequence(&self) {
        if let Err(e) = self.execute_prepare_phases().await {
            let user_error = UserFacingError::from(&e);
            match user_error.severity {
                ErrorSeverity::Blocked => {
                    self.set_state(SupervisorState::FailedBlocked {
                        error: user_error,
                    });
                }
                ErrorSeverity::Recoverable => {
                    self.set_state(SupervisorState::FailedRecoverable {
                        error: user_error,
                    });
                }
            }
        }
    }

    async fn run_start_sequence(&self) {
        let plan = self.build_runtime_plan();

        self.set_state(SupervisorState::StartingServices);
        if let Err(e) = self.adapter.start_app(&plan).await {
            self.fail_recoverable(e);
            return;
        }

        self.set_state(SupervisorState::StartingProxy);
        let proxy_port = match crate::proxy::allocate_proxy_port() {
            Ok(p) => p,
            Err(_e) => {
                self.fail_recoverable(CrateRunError::ProxyBindFailed { port: 0 });
                return;
            }
        };

        let proxy = ReverseProxy::new(&self.manifest, &plan);
        let proxy_handle = tokio::spawn(async move {
            if let Err(e) = proxy.run(proxy_port).await {
                tracing::error!("proxy error: {}", e);
            }
        });

        self.set_state(SupervisorState::WaitingForHealth);
        let checker = HealthChecker::new(
            &self.manifest,
            &plan,
        );

        match checker.wait_until_ready().await {
            Ok(()) => {
                let app_url = format!("http://127.0.0.1:{}/", proxy_port);
                self.set_state(SupervisorState::Ready { app_url });
            }
            Err(e) => {
                self.fail_recoverable(e);
                proxy_handle.abort();
            }
        }
    }

    async fn execute_prepare_phases(&self) -> Result<(), CrateRunError> {
        self.set_state(SupervisorState::CheckingSystem);
        let detection = self.adapter.detect().await?;
        if !detection.available {
            return Err(CrateRunError::RuntimeNotFound(
                detection
                    .blocker
                    .unwrap_or_else(|| "container runtime not available".into()),
            ));
        }

        self.set_state(SupervisorState::PreparingRuntime);
        self.adapter.prepare().await?;

        self.set_state(SupervisorState::ImportingImages);
        for service in &self.manifest.services {
            let archive_path = self.reader.image_path(&service.image_archive);
            if archive_path.exists() {
                self.adapter
                    .import_image(&archive_path, &service.name)
                    .await?;
            } else {
                tracing::warn!(
                    "image archive not found: {} (skipping for dev mode)",
                    archive_path.display()
                );
            }
        }

        self.set_state(SupervisorState::CreatingNetwork);
        self.adapter.create_network(&self.app_id).await?;

        self.set_state(SupervisorState::CreatingVolumes);
        for (vol_name, _vol_config) in &self.manifest.volumes {
            self.adapter
                .create_volume(&self.app_id, vol_name)
                .await?;
        }

        self.run_start_sequence().await;
        Ok(())
    }

    fn build_runtime_plan(&self) -> RuntimePlan {
        let mut base_port = 49200u16;
        let services: Vec<PlannedService> = self
            .manifest
            .services
            .iter()
            .map(|svc| {
                let host_port = base_port;
                base_port += 1;

                let volumes: Vec<VolumeMount> = self
                    .manifest
                    .volumes
                    .keys()
                    .filter(|_vn| {
                        // Match volumes by checking if any service in the compose references them
                        // For MVP, we pass all declared volumes to all services that might use them
                        false
                    })
                    .map(|vn| VolumeMount {
                        volume_name: format!(
                            "craterun-{}-{}",
                            self.app_id.as_str().replace('.', "-"),
                            vn
                        ),
                        container_path: String::new(),
                    })
                    .collect();

                PlannedService {
                    name: svc.name.clone(),
                    image_ref: ImageRef {
                        name: svc.name.clone(),
                        digest: svc.image_digest.clone(),
                        archive_path: Some(svc.image_archive.clone()),
                    },
                    container_port: svc.container_port,
                    host_port,
                    env: svc.env.clone(),
                    depends_on: svc.depends_on.clone(),
                    volumes,
                    restart_policy: svc.restart_policy,
                }
            })
            .collect();

        RuntimePlan {
            app_id: self.app_id.clone(),
            services,
            network_name: format!("craterun-{}", self.app_id.as_str().replace('.', "-")),
        }
    }

    fn set_state(&self, state: SupervisorState) {
        let _ = self.state_tx.send(state);
    }

    fn fail_recoverable(&self, err: CrateRunError) {
        let user_error = UserFacingError::from(&err);
        self.set_state(SupervisorState::FailedRecoverable {
            error: user_error,
        });
    }
}
