//! WSL2 + containerd runtime adapter for consumer Windows installs.
//!
//! Each app gets its own WSL2 distro (`bento-<appId>`) with containerd
//! and nerdctl pre-installed. This avoids any dependency on Docker Desktop.
//!
//! All container operations shell out through `wsl.exe -d <distro> -- nerdctl ...`.
//! The distro is imported from a base tarball bundled with the app.
//!
//! This is the highest-risk component in the system. Potential blockers:
//! - Hardware virtualization disabled in BIOS
//! - WSL2 kernel not installed
//! - Windows feature requires reboot
//! - Enterprise group policy blocking WSL
//! - Windows Defender interfering with containerd
//!
//! See docs/decisions.md ADR-009 for rationale.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::process::Command;

use bento_core::types::ContainerState;
use bento_core::{AppId, BentoError};

use crate::adapter::RuntimeAdapter;
use crate::types::{
    ContainerStatus, ImageRef, LogLine, RemoveOptions, RuntimeDetectionResult, RuntimePlan,
};

/// Namespace prefix for all Bento-managed WSL distros.
const DISTRO_PREFIX: &str = "bento-";

/// Script that bootstraps containerd + nerdctl inside a fresh WSL distro.
/// Runs once during the `prepare()` phase on first launch.
/// Used by prepare() when distro import is fully wired (see TODO in prepare).
#[allow(dead_code)]
const BOOTSTRAP_SCRIPT: &str = r#"#!/bin/sh
set -e

# Skip if containerd is already installed
if command -v nerdctl >/dev/null 2>&1; then
    echo "nerdctl already available"
    exit 0
fi

export DEBIAN_FRONTEND=noninteractive

apt-get update -qq
apt-get install -y -qq containerd iptables > /dev/null 2>&1

# Install nerdctl (standalone binary, no Docker dependency)
NERDCTL_VERSION="2.0.3"
ARCH=$(uname -m)
case "$ARCH" in
    x86_64) ARCH="amd64" ;;
    aarch64) ARCH="arm64" ;;
esac

curl -fsSL "https://github.com/containerd/nerdctl/releases/download/v${NERDCTL_VERSION}/nerdctl-${NERDCTL_VERSION}-linux-${ARCH}.tar.gz" \
    | tar -xz -C /usr/local/bin nerdctl

# Configure containerd with default settings
mkdir -p /etc/containerd
containerd config default > /etc/containerd/config.toml

echo "Bootstrap complete"
"#;

pub struct WslContainerdAdapter {
    /// Filesystem path to the base distro tarball (bundled with the app).
    /// Used by `wsl --import` to create the per-app distro.
    base_distro_path: Option<PathBuf>,
}

impl WslContainerdAdapter {
    pub fn new() -> Self {
        Self {
            base_distro_path: None,
        }
    }

    pub fn with_base_distro(base_distro_path: PathBuf) -> Self {
        Self {
            base_distro_path: Some(base_distro_path),
        }
    }

    fn distro_name(app_id: &AppId) -> String {
        format!("{}{}", DISTRO_PREFIX, app_id.as_str().replace('.', "-"))
    }

    fn container_name(app_id: &AppId, service: &str) -> String {
        format!("{}-{}", Self::distro_name(app_id), service)
    }

    /// Run a command inside the app's WSL distro via `wsl.exe -d <distro> -- <cmd>`.
    async fn wsl_exec(distro: &str, args: &[&str]) -> Result<std::process::Output, BentoError> {
        let mut cmd_args = vec!["-d", distro, "--"];
        cmd_args.extend(args);

        Command::new("wsl.exe")
            .args(&cmd_args)
            .output()
            .await
            .map_err(|e| BentoError::RuntimeError(format!("wsl exec failed: {}", e)))
    }

    /// Check whether a specific WSL distro exists.
    async fn distro_exists(distro: &str) -> bool {
        let output = Command::new("wsl.exe")
            .args(["-l", "-q"])
            .output()
            .await;

        match output {
            Ok(o) => {
                // WSL outputs UTF-16LE on Windows. Decode and search.
                let text = String::from_utf16_lossy(
                    &o.stdout
                        .chunks_exact(2)
                        .map(|c| u16::from_le_bytes([c[0], c[1]]))
                        .collect::<Vec<_>>(),
                );
                text.lines().any(|line| line.trim() == distro)
            }
            Err(_) => false,
        }
    }

    /// Ensure containerd is running inside the distro.
    async fn ensure_containerd(distro: &str) -> Result<(), BentoError> {
        // containerd may already be running from a previous session
        let check = Self::wsl_exec(distro, &["pgrep", "-x", "containerd"]).await?;
        if check.status.success() {
            return Ok(());
        }

        // Start containerd in the background
        let start = Self::wsl_exec(
            distro,
            &["sh", "-c", "containerd > /var/log/containerd.log 2>&1 &"],
        )
        .await?;

        if !start.status.success() {
            return Err(BentoError::RuntimeError(
                "failed to start containerd".into(),
            ));
        }

        // Brief wait for containerd socket to appear
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        Ok(())
    }
}

#[async_trait]
impl RuntimeAdapter for WslContainerdAdapter {
    async fn detect(&self) -> Result<RuntimeDetectionResult, BentoError> {
        // Step 1: Check if wsl.exe exists and responds
        let wsl_status = Command::new("wsl.exe")
            .args(["--status"])
            .output()
            .await;

        match wsl_status {
            Ok(o) if o.status.success() => {}
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                // WSL not installed or virtualization disabled
                if stderr.contains("enable") || stderr.contains("Virtual Machine Platform") {
                    return Ok(RuntimeDetectionResult {
                        available: false,
                        version: None,
                        requires_preparation: false,
                        blocker: Some(
                            "WSL2 requires Virtual Machine Platform. Enable it in Windows Features."
                                .into(),
                        ),
                    });
                }
                return Ok(RuntimeDetectionResult {
                    available: false,
                    version: None,
                    requires_preparation: false,
                    blocker: Some(format!("WSL2 not available: {}", stderr.trim())),
                });
            }
            Err(e) => {
                return Ok(RuntimeDetectionResult {
                    available: false,
                    version: None,
                    requires_preparation: false,
                    blocker: Some(format!("WSL2 not installed: {}", e)),
                });
            }
        }

        // Step 2: Check WSL version (need WSL2, not WSL1)
        let version_output = Command::new("wsl.exe")
            .args(["--version"])
            .output()
            .await;

        let version = version_output.ok().map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string()
        });

        Ok(RuntimeDetectionResult {
            available: true,
            version,
            requires_preparation: true,
            blocker: None,
        })
    }

    async fn prepare(&self) -> Result<(), BentoError> {
        // Import a base distro if it doesn't already exist.
        // For development, we assume a suitable distro is already available
        // or we import from the bundled tarball.
        //
        // In production, the .appcrate bundle includes a minimal Alpine/Debian
        // tarball at runtime-assets/bento-base-distro.tar.gz

        // For now, log what would happen. Full distro import requires:
        // 1. wsl --import <distro> <installDir> <tarball> --version 2
        // 2. Run bootstrap script to install containerd + nerdctl
        // 3. Start containerd

        if let Some(ref tarball) = self.base_distro_path {
            tracing::info!(
                "WSL distro import from {} (not yet implemented for MVP)",
                tarball.display()
            );
        }

        Ok(())
    }

    async fn import_image(
        &self,
        archive_path: &Path,
        image_name: &str,
    ) -> Result<ImageRef, BentoError> {
        // Convert Windows path to WSL-accessible path
        let wsl_path = windows_to_wsl_path(archive_path);

        // nerdctl load reads OCI/Docker archives
        let output = Command::new("wsl.exe")
            .args([
                "-d",
                // The distro name is derived from context; for import_image
                // we need the app_id. This is a limitation — the adapter
                // doesn't know the app_id here. We use a default distro for now.
                "bento-default",
                "--",
                "nerdctl",
                "load",
                "-i",
                &wsl_path,
            ])
            .output()
            .await
            .map_err(|e| BentoError::ImageImportFailed {
                service: image_name.to_string(),
                detail: format!("wsl nerdctl load failed: {}", e),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BentoError::ImageImportFailed {
                service: image_name.to_string(),
                detail: format!("nerdctl load: {}", stderr),
            });
        }

        Ok(ImageRef {
            name: image_name.to_string(),
            digest: None,
            archive_path: Some(archive_path.to_string_lossy().to_string()),
        })
    }

    async fn start_app(&self, plan: &RuntimePlan) -> Result<(), BentoError> {
        let distro = Self::distro_name(&plan.app_id);

        Self::ensure_containerd(&distro).await?;

        for service in &plan.services {
            let container = Self::container_name(&plan.app_id, &service.name);

            let mut nerdctl_args = vec![
                "nerdctl".to_string(),
                "run".to_string(),
                "-d".to_string(),
                "--name".to_string(),
                container.clone(),
                "--net".to_string(),
                plan.network_name.clone(),
                // Port forward: WSL2 auto-forwards ports bound to 0.0.0.0 inside
                // the distro to the Windows host on the same port
                "-p".to_string(),
                format!("{}:{}", service.host_port, service.container_port),
            ];

            for (key, val) in &service.env {
                nerdctl_args.push("-e".to_string());
                nerdctl_args.push(format!("{}={}", key, val));
            }

            for vol in &service.volumes {
                nerdctl_args.push("-v".to_string());
                nerdctl_args.push(format!("{}:{}", vol.volume_name, vol.container_path));
            }

            nerdctl_args.push(service.image_ref.name.clone());

            let args_refs: Vec<&str> = nerdctl_args.iter().map(|s| s.as_str()).collect();
            let output = Self::wsl_exec(&distro, &args_refs).await?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(BentoError::ContainerStartFailed {
                    service: service.name.clone(),
                    detail: format!("nerdctl run: {}", stderr),
                });
            }
        }

        Ok(())
    }

    async fn stop_app(&self, app_id: &AppId) -> Result<(), BentoError> {
        let distro = Self::distro_name(app_id);

        if !Self::distro_exists(&distro).await {
            return Ok(());
        }

        // List all containers with our prefix and stop them
        let prefix = Self::container_name(app_id, "");
        let output = Self::wsl_exec(
            &distro,
            &[
                "nerdctl",
                "ps",
                "-a",
                "--filter",
                &format!("name={}", prefix),
                "--format",
                "{{.Names}}",
            ],
        )
        .await?;

        let names = String::from_utf8_lossy(&output.stdout);
        for name in names.lines() {
            let name = name.trim();
            if !name.is_empty() {
                let _ = Self::wsl_exec(&distro, &["nerdctl", "stop", name]).await;
                let _ = Self::wsl_exec(&distro, &["nerdctl", "rm", "-f", name]).await;
            }
        }

        Ok(())
    }

    async fn get_container_status(
        &self,
        app_id: &AppId,
        service: &str,
    ) -> Result<ContainerStatus, BentoError> {
        let distro = Self::distro_name(app_id);
        let container = Self::container_name(app_id, service);

        let output = Self::wsl_exec(
            &distro,
            &[
                "nerdctl",
                "inspect",
                "--format",
                "{{.State.Status}}",
                &container,
            ],
        )
        .await?;

        let status_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let state = match status_str.as_str() {
            "running" => ContainerState::Running,
            "created" | "restarting" => ContainerState::Starting,
            "paused" | "exited" | "dead" => ContainerState::Stopped,
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
        let distro = Self::distro_name(app_id);
        let tail_str = tail.unwrap_or(100).to_string();
        let mut logs = Vec::new();

        let containers: Vec<String> = if let Some(svc) = service {
            vec![Self::container_name(app_id, svc)]
        } else {
            // List all containers for this app
            let prefix = Self::container_name(app_id, "");
            let output = Self::wsl_exec(
                &distro,
                &[
                    "nerdctl",
                    "ps",
                    "-a",
                    "--filter",
                    &format!("name={}", prefix),
                    "--format",
                    "{{.Names}}",
                ],
            )
            .await?;
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        };

        for container in containers {
            let output = Self::wsl_exec(
                &distro,
                &["nerdctl", "logs", "--tail", &tail_str, &container],
            )
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

            // Combine stdout and stderr — nerdctl logs sends app output to both
            let combined = [&output.stdout[..], &output.stderr[..]].concat();
            for line in String::from_utf8_lossy(&combined).lines() {
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
        let distro = Self::distro_name(app_id);
        let network = format!("bento-{}", app_id.as_str().replace('.', "-"));

        let output =
            Self::wsl_exec(&distro, &["nerdctl", "network", "create", &network]).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("already exists") {
                return Err(BentoError::RuntimeError(format!(
                    "nerdctl network create '{}': {}",
                    network, stderr
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
        let distro = Self::distro_name(app_id);
        let full_name = format!(
            "bento-{}-{}",
            app_id.as_str().replace('.', "-"),
            volume_name
        );

        let output =
            Self::wsl_exec(&distro, &["nerdctl", "volume", "create", &full_name]).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BentoError::RuntimeError(format!(
                "nerdctl volume create '{}': {}",
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
        let distro = Self::distro_name(app_id);

        // Stop all containers first
        self.stop_app(app_id).await?;

        // Remove the network
        let network = format!("bento-{}", app_id.as_str().replace('.', "-"));
        let _ = Self::wsl_exec(&distro, &["nerdctl", "network", "rm", &network]).await;

        if options.remove_volumes {
            // Remove all volumes with our prefix
            let prefix = format!("bento-{}-", app_id.as_str().replace('.', "-"));
            let output = Self::wsl_exec(
                &distro,
                &[
                    "nerdctl",
                    "volume",
                    "ls",
                    "--format",
                    "{{.Name}}",
                ],
            )
            .await
            .unwrap_or_else(|_| std::process::Output {
                status: std::process::ExitStatus::default(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            });

            for vol in String::from_utf8_lossy(&output.stdout).lines() {
                let vol = vol.trim();
                if vol.starts_with(&prefix) {
                    let _ =
                        Self::wsl_exec(&distro, &["nerdctl", "volume", "rm", vol]).await;
                }
            }
        }

        if options.remove_runtime {
            // Unregister the entire WSL distro — this is the nuclear option.
            // Only used during full uninstall. Deletes the distro's VHDX file.
            let output = Command::new("wsl.exe")
                .args(["--unregister", &distro])
                .output()
                .await
                .map_err(|e| {
                    BentoError::RuntimeError(format!("wsl unregister failed: {}", e))
                })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::warn!("WSL unregister '{}' failed: {}", distro, stderr);
            }
        }

        Ok(())
    }

    fn adapter_name(&self) -> &'static str {
        "wsl-containerd"
    }
}

/// Convert a Windows path (C:\foo\bar) to a WSL-accessible path (/mnt/c/foo/bar).
fn windows_to_wsl_path(path: &Path) -> String {
    let path_str = path.to_string_lossy();

    // Handle UNC-style paths from canonicalize (\\?\C:\...)
    let cleaned = path_str
        .strip_prefix(r"\\?\")
        .unwrap_or(&path_str);

    // Convert C:\foo\bar -> /mnt/c/foo/bar
    if cleaned.len() >= 2 && cleaned.as_bytes()[1] == b':' {
        let drive = (cleaned.as_bytes()[0] as char).to_ascii_lowercase();
        let rest = &cleaned[2..].replace('\\', "/");
        format!("/mnt/{}{}", drive, rest)
    } else {
        cleaned.replace('\\', "/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_path_conversion() {
        let path = Path::new(r"C:\Users\user\bundle\images\web.tar.zst");
        assert_eq!(
            windows_to_wsl_path(path),
            "/mnt/c/Users/user/bundle/images/web.tar.zst"
        );
    }

    #[test]
    fn windows_unc_path_conversion() {
        let path = Path::new(r"\\?\C:\Users\user\file.txt");
        assert_eq!(
            windows_to_wsl_path(path),
            "/mnt/c/Users/user/file.txt"
        );
    }

    #[test]
    fn distro_naming() {
        let id = AppId::new("com.example.photobooth").unwrap();
        assert_eq!(
            WslContainerdAdapter::distro_name(&id),
            "bento-com-example-photobooth"
        );
    }

    #[test]
    fn container_naming() {
        let id = AppId::new("com.example.app").unwrap();
        assert_eq!(
            WslContainerdAdapter::container_name(&id, "web"),
            "bento-com-example-app-web"
        );
    }
}
