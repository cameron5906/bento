use std::path::Path;

use async_trait::async_trait;
use tokio::process::Command;

use bento_core::types::ContainerState;
use bento_core::{AppId, BentoError};

/// Create a docker Command with console window hidden on Windows
fn docker_cmd() -> Command {
    let mut cmd = docker_cmd();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW prevents console flashes when running docker CLI
        cmd.creation_flags(0x08000000);
    }
    cmd
}

use crate::adapter::RuntimeAdapter;
use crate::types::{ContainerStatus, ImageRef, LogLine, RemoveOptions, RuntimeDetectionResult, RuntimePlan};

pub struct ExistingDockerAdapter;

impl ExistingDockerAdapter {
    pub fn new() -> Self {
        Self
    }

    fn container_name(app_id: &AppId, service: &str) -> String {
        format!("bento-{}-{}", app_id.as_str().replace('.', "-"), service)
    }

    fn network_name(app_id: &AppId) -> String {
        format!("bento-{}", app_id.as_str().replace('.', "-"))
    }
}

#[async_trait]
impl RuntimeAdapter for ExistingDockerAdapter {
    async fn detect(&self) -> Result<RuntimeDetectionResult, BentoError> {
        Ok(crate::detect::detect_docker().await)
    }

    async fn prepare(&self) -> Result<(), BentoError> {
        Ok(())
    }

    async fn import_image(
        &self,
        archive_path: &Path,
        image_name: &str,
    ) -> Result<ImageRef, BentoError> {
        let output = docker_cmd()
            .args(["load", "-i", &archive_path.to_string_lossy()])
            .output()
            .await
            .map_err(|e| BentoError::ImageImportFailed {
                service: image_name.to_string(),
                detail: format!("failed to run docker load: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BentoError::ImageImportFailed {
                service: image_name.to_string(),
                detail: stderr.to_string(),
            });
        }

        Ok(ImageRef {
            name: image_name.to_string(),
            digest: None,
            archive_path: Some(archive_path.to_string_lossy().to_string()),
        })
    }

    async fn start_app(&self, plan: &RuntimePlan) -> Result<(), BentoError> {
        for service in &plan.services {
            let container = Self::container_name(&plan.app_id, &service.name);

            // Remove any leftover container from a previous run so the name is free
            let _ = docker_cmd()
                .args(["rm", "-f", &container])
                .output()
                .await;

            let mut args = vec![
                "run".to_string(),
                "-d".to_string(),
                "--name".to_string(),
                container.clone(),
                "--network".to_string(),
                plan.network_name.clone(),
                "--network-alias".to_string(),
                service.name.clone(),
            ];

            // Only bind ports for services that expose them (db has port 0)
            if service.container_port > 0 {
                args.push("-p".to_string());
                args.push(format!(
                    "127.0.0.1:{}:{}",
                    service.host_port, service.container_port
                ));
            }

            for (key, val) in &service.env {
                args.push("-e".to_string());
                args.push(format!("{}={}", key, val));
            }

            for vol in &service.volumes {
                args.push("-v".to_string());
                args.push(format!("{}:{}", vol.volume_name, vol.container_path));
            }

            args.push(service.image_ref.name.clone());

            let output = docker_cmd()
                .args(&args)
                .output()
                .await
                .map_err(|e| BentoError::ContainerStartFailed {
                    service: service.name.clone(),
                    detail: format!("failed to run docker: {}", e),
                })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(BentoError::ContainerStartFailed {
                    service: service.name.clone(),
                    detail: stderr.to_string(),
                });
            }
        }

        Ok(())
    }

    async fn stop_app(&self, app_id: &AppId) -> Result<(), BentoError> {
        let prefix = format!("bento-{}", app_id.as_str().replace('.', "-"));
        let output = docker_cmd()
            .args(["ps", "-a", "--filter", &format!("name={}", prefix), "--format", "{{.Names}}"])
            .output()
            .await
            .map_err(|e| BentoError::RuntimeError(format!("docker ps failed: {}", e)))?;

        let names = String::from_utf8_lossy(&output.stdout);
        for name in names.lines() {
            let name = name.trim();
            if !name.is_empty() {
                let _ = docker_cmd()
                    .args(["stop", name])
                    .output()
                    .await;
                let _ = docker_cmd()
                    .args(["rm", "-f", name])
                    .output()
                    .await;
            }
        }

        Ok(())
    }

    async fn get_container_status(
        &self,
        app_id: &AppId,
        service: &str,
    ) -> Result<ContainerStatus, BentoError> {
        let container = Self::container_name(app_id, service);
        let output = docker_cmd()
            .args(["inspect", "--format", "{{.State.Status}}", &container])
            .output()
            .await
            .map_err(|e| BentoError::RuntimeError(format!("docker inspect failed: {}", e)))?;

        let status_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let state = match status_str.as_str() {
            "running" => ContainerState::Running,
            "created" | "restarting" => ContainerState::Starting,
            "exited" | "dead" => ContainerState::Stopped,
            _ => ContainerState::Failed,
        };

        Ok(ContainerStatus {
            name: service.to_string(),
            state,
            exit_code: None,
        })
    }

    async fn get_logs(
        &self,
        app_id: &AppId,
        service: Option<&str>,
        tail: Option<usize>,
    ) -> Result<Vec<LogLine>, BentoError> {
        let containers: Vec<String> = if let Some(svc) = service {
            vec![Self::container_name(app_id, svc)]
        } else {
            let prefix = format!("bento-{}", app_id.as_str().replace('.', "-"));
            let output = docker_cmd()
                .args(["ps", "-a", "--filter", &format!("name={}", prefix), "--format", "{{.Names}}"])
                .output()
                .await
                .map_err(|e| BentoError::RuntimeError(e.to_string()))?;
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        };

        let mut logs = Vec::new();
        let tail_str = tail.unwrap_or(100).to_string();

        for container in containers {
            let output = docker_cmd()
                .args(["logs", "--tail", &tail_str, &container])
                .output()
                .await
                .unwrap_or_else(|_| std::process::Output {
                    status: std::process::ExitStatus::default(),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                });

            let svc_name = container
                .rsplit('-')
                .next()
                .unwrap_or(&container)
                .to_string();

            for line in String::from_utf8_lossy(&output.stdout).lines() {
                logs.push(LogLine {
                    timestamp: String::new(),
                    service: svc_name.clone(),
                    message: line.to_string(),
                });
            }
        }

        Ok(logs)
    }

    async fn create_network(&self, app_id: &AppId) -> Result<(), BentoError> {
        let name = Self::network_name(app_id);
        let output = docker_cmd()
            .args(["network", "create", &name])
            .output()
            .await
            .map_err(|e| BentoError::RuntimeError(format!("docker network create failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("already exists") {
                return Err(BentoError::RuntimeError(format!(
                    "failed to create network '{}': {}",
                    name, stderr
                )));
            }
        }

        Ok(())
    }

    async fn create_volume(
        &self,
        app_id: &AppId,
        volume_name: &str,
    ) -> Result<(), BentoError> {
        let full_name = format!(
            "bento-{}-{}",
            app_id.as_str().replace('.', "-"),
            volume_name
        );
        let output = docker_cmd()
            .args(["volume", "create", &full_name])
            .output()
            .await
            .map_err(|e| BentoError::RuntimeError(format!("docker volume create failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BentoError::RuntimeError(format!(
                "failed to create volume '{}': {}",
                full_name, stderr
            )));
        }

        Ok(())
    }

    async fn remove_app(
        &self,
        app_id: &AppId,
        options: RemoveOptions,
    ) -> Result<(), BentoError> {
        self.stop_app(app_id).await?;

        let network = Self::network_name(app_id);
        let _ = docker_cmd()
            .args(["network", "rm", &network])
            .output()
            .await;

        if options.remove_volumes {
            let prefix = format!("bento-{}", app_id.as_str().replace('.', "-"));
            let output = docker_cmd()
                .args(["volume", "ls", "--filter", &format!("name={}", prefix), "--format", "{{.Name}}"])
                .output()
                .await
                .unwrap_or_else(|_| std::process::Output {
                    status: std::process::ExitStatus::default(),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                });

            for vol in String::from_utf8_lossy(&output.stdout).lines() {
                let vol = vol.trim();
                if !vol.is_empty() {
                    let _ = docker_cmd()
                        .args(["volume", "rm", vol])
                        .output()
                        .await;
                }
            }
        }

        Ok(())
    }

    fn adapter_name(&self) -> &'static str {
        "existing-docker"
    }
}
