//! Diagnostics export for consumer support.
//!
//! Collects non-sensitive system info, supervisor state, logs, and health
//! check results into a JSON bundle. Explicitly excludes secrets, raw
//! environment variables, tokens, and user data volumes.

use serde::Serialize;

use bento_bundle::manifest::compiled_manifest::CompiledManifest;
use bento_core::SupervisorState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsBundle {
    pub app_id: String,
    pub app_version: String,
    pub supervisor_state: SupervisorState,
    pub system_info: SystemInfo,
    pub runtime_info: RuntimeInfo,
    pub service_logs: Vec<ServiceLogEntry>,
    pub health_check: HealthCheckInfo,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub architecture: String,
    pub available_memory_mb: u64,
    pub available_disk_mb: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    pub adapter: String,
    pub adapter_available: bool,
    pub adapter_version: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceLogEntry {
    pub service: String,
    /// Last N log lines — sanitized, no env vars or secrets
    pub lines: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheckInfo {
    pub service: String,
    pub path: String,
    pub timeout_seconds: u32,
    pub last_status: Option<String>,
}

impl DiagnosticsBundle {
    pub fn collect(
        manifest: &CompiledManifest,
        state: &SupervisorState,
        adapter_name: &str,
        logs: Vec<ServiceLogEntry>,
    ) -> Self {
        Self {
            app_id: manifest.app.id.clone(),
            app_version: manifest.app.version.clone(),
            supervisor_state: state.clone(),
            system_info: collect_system_info(),
            runtime_info: RuntimeInfo {
                adapter: adapter_name.to_string(),
                adapter_available: true,
                adapter_version: None,
            },
            service_logs: logs,
            health_check: HealthCheckInfo {
                service: manifest.health.ready.service.clone(),
                path: manifest.health.ready.path.clone(),
                timeout_seconds: manifest.health.ready.timeout_seconds,
                last_status: None,
            },
        }
    }
}

fn collect_system_info() -> SystemInfo {
    SystemInfo {
        os: std::env::consts::OS.to_string(),
        os_version: get_os_version(),
        architecture: std::env::consts::ARCH.to_string(),
        available_memory_mb: get_available_memory_mb(),
        available_disk_mb: get_available_disk_mb(),
    }
}

fn get_os_version() -> String {
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/c", "ver"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".into())
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("uname")
            .arg("-r")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".into())
    }
}

fn get_available_memory_mb() -> u64 {
    // Rough estimate via systeminfo would be slow; return 0 for now.
    // A proper implementation would use Windows API (GlobalMemoryStatusEx).
    0
}

fn get_available_disk_mb() -> u64 {
    // Would use GetDiskFreeSpaceExW on Windows.
    // Returning 0 as placeholder; safe since diagnostics are informational.
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use bento_bundle::manifest::compiled_manifest::*;
    use bento_core::types::*;
    use indexmap::IndexMap;

    fn sample_manifest() -> CompiledManifest {
        CompiledManifest {
            schema_version: "0.1".into(),
            app: CompiledAppInfo {
                id: "com.example.test".into(),
                name: "Test".into(),
                version: "1.0.0".into(),
            },
            runtime: RuntimeTarget {
                target: "windows-x64".into(),
                architecture: "linux/amd64".into(),
            },
            services: vec![],
            routes: vec![],
            health: CompiledHealth {
                ready: CompiledHealthCheck {
                    service: "api".into(),
                    path: "/health".into(),
                    timeout_seconds: 120,
                },
            },
            volumes: IndexMap::new(),
            window: CompiledWindow {
                title: "Test".into(),
                width: 800,
                height: 600,
                entry: "/".into(),
            },
            lifecycle: CompiledLifecycle {
                on_window_open: LifecycleAction::StartServices,
                on_window_close: LifecycleAction::StopServices,
            },
            install: CompiledInstall {
                mode: InstallMode::Consumer,
                ask_questions: false,
            },
        }
    }

    #[test]
    fn diagnostics_bundle_serializes() {
        let manifest = sample_manifest();
        let state = SupervisorState::Stopped;
        let bundle = DiagnosticsBundle::collect(
            &manifest,
            &state,
            "existing-docker",
            vec![ServiceLogEntry {
                service: "api".into(),
                lines: vec!["server started on :8080".into()],
            }],
        );

        let json = serde_json::to_string(&bundle).unwrap();
        assert!(json.contains("com.example.test"));
        assert!(json.contains("existing-docker"));
        assert!(json.contains("server started"));
    }
}
