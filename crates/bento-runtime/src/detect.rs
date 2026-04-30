use std::sync::Arc;

use crate::adapter::RuntimeAdapter;
use crate::adapters::existing_docker::ExistingDockerAdapter;
#[cfg(windows)]
use crate::adapters::wsl_containerd::WslContainerdAdapter;
use crate::types::RuntimeDetectionResult;

pub async fn detect_docker() -> RuntimeDetectionResult {
    let output = tokio::process::Command::new("docker")
        .args(["version", "--format", "{{.Server.Version}}"])
        .output()
        .await;

    match output {
        Ok(o) if o.status.success() => {
            let version = String::from_utf8_lossy(&o.stdout).trim().to_string();
            RuntimeDetectionResult {
                available: true,
                version: Some(version),
                requires_preparation: false,
                blocker: None,
            }
        }
        _ => RuntimeDetectionResult {
            available: false,
            version: None,
            requires_preparation: false,
            blocker: Some("Docker is not installed or not running".into()),
        },
    }
}

pub async fn detect_podman() -> RuntimeDetectionResult {
    let output = tokio::process::Command::new("podman")
        .args(["version", "--format", "{{.Client.Version}}"])
        .output()
        .await;

    match output {
        Ok(o) if o.status.success() => {
            let version = String::from_utf8_lossy(&o.stdout).trim().to_string();
            RuntimeDetectionResult {
                available: true,
                version: Some(version),
                requires_preparation: false,
                blocker: None,
            }
        }
        _ => RuntimeDetectionResult {
            available: false,
            version: None,
            requires_preparation: false,
            blocker: Some("Podman is not installed".into()),
        },
    }
}

pub async fn detect_wsl() -> RuntimeDetectionResult {
    #[cfg(windows)]
    {
        let output = tokio::process::Command::new("wsl.exe")
            .args(["--status"])
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => RuntimeDetectionResult {
                available: true,
                version: None,
                requires_preparation: true,
                blocker: None,
            },
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                RuntimeDetectionResult {
                    available: false,
                    version: None,
                    requires_preparation: false,
                    blocker: Some(format!("WSL2 not available: {}", stderr.trim())),
                }
            }
            Err(e) => RuntimeDetectionResult {
                available: false,
                version: None,
                requires_preparation: false,
                blocker: Some(format!("WSL2 not installed: {}", e)),
            },
        }
    }

    #[cfg(not(windows))]
    {
        RuntimeDetectionResult {
            available: false,
            version: None,
            requires_preparation: false,
            blocker: Some("WSL2 is only available on Windows".into()),
        }
    }
}

/// Select the best available runtime adapter for this system.
/// - Windows: prefers WSL2+containerd for consumer, Docker for dev
/// - macOS/Linux: uses existing Docker (or Podman via Docker-compat shim)
pub async fn select_adapter(prefer_consumer: bool) -> (Arc<dyn RuntimeAdapter>, &'static str) {
    #[cfg(windows)]
    {
        if prefer_consumer {
            let wsl = detect_wsl().await;
            if wsl.available {
                return (Arc::new(WslContainerdAdapter::new()), "wsl-containerd");
            }
        }
    }

    let docker = detect_docker().await;
    if docker.available {
        return (Arc::new(ExistingDockerAdapter::new()), "existing-docker");
    }

    // Fallback: return Docker adapter anyway — it will fail with a clear error
    // at detect() time rather than silently
    #[cfg(windows)]
    {
        return (Arc::new(WslContainerdAdapter::new()), "wsl-containerd");
    }
    #[cfg(not(windows))]
    {
        return (Arc::new(ExistingDockerAdapter::new()), "existing-docker");
    }
}
