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
