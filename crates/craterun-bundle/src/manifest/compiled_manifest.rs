use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use craterun_core::types::{
    InstallMode, LifecycleAction, RestartPolicy, ServiceRole, VolumeDurability,
};

use super::app_manifest::AppManifest;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledManifest {
    pub schema_version: String,
    pub app: CompiledAppInfo,
    pub runtime: RuntimeTarget,
    pub services: Vec<ServiceEntry>,
    pub routes: Vec<CompiledRoute>,
    pub health: CompiledHealth,
    pub volumes: IndexMap<String, CompiledVolume>,
    pub window: CompiledWindow,
    pub lifecycle: CompiledLifecycle,
    pub install: CompiledInstall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledAppInfo {
    pub id: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeTarget {
    pub target: String,
    pub architecture: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceEntry {
    pub name: String,
    /// Docker image tag used during build (e.g. "hello-web-api-web:latest").
    /// The supervisor uses this to run containers after importing from the archive.
    #[serde(default)]
    pub image_tag: Option<String>,
    pub image_archive: String,
    #[serde(default)]
    pub image_digest: Option<String>,
    pub container_port: u16,
    pub role: ServiceRole,
    #[serde(default)]
    pub env: IndexMap<String, String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub restart_policy: RestartPolicy,
    /// Named volume mounts for this service (e.g. "db-data" -> "/var/lib/postgresql/data")
    #[serde(default)]
    pub volume_mounts: Vec<ServiceVolumeMount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceVolumeMount {
    pub name: String,
    pub mount_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledRoute {
    pub path: String,
    pub service: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledHealth {
    pub ready: CompiledHealthCheck,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledHealthCheck {
    pub service: String,
    pub path: String,
    pub timeout_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledVolume {
    pub durability: VolumeDurability,
    pub backup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledWindow {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub entry: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledLifecycle {
    pub on_window_open: LifecycleAction,
    pub on_window_close: LifecycleAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompiledInstall {
    pub mode: InstallMode,
    pub ask_questions: bool,
}

impl CompiledManifest {
    pub fn from_app_manifest(
        manifest: &AppManifest,
        services: Vec<ServiceEntry>,
        target: &str,
    ) -> Self {
        Self {
            schema_version: "0.1".into(),
            app: CompiledAppInfo {
                id: manifest.app.id.as_str().to_string(),
                name: manifest.app.name.clone(),
                version: manifest.app.version.clone(),
            },
            runtime: RuntimeTarget {
                target: target.to_string(),
                architecture: "linux/amd64".into(),
            },
            services,
            routes: manifest
                .routes
                .iter()
                .map(|(path, target)| CompiledRoute {
                    path: path.clone(),
                    service: target.service.clone(),
                    port: target.port,
                })
                .collect(),
            health: CompiledHealth {
                ready: CompiledHealthCheck {
                    service: manifest.health.ready.service.clone(),
                    path: manifest.health.ready.path.clone(),
                    timeout_seconds: manifest.health.ready.timeout_seconds,
                },
            },
            volumes: manifest
                .volumes
                .iter()
                .map(|(name, vol)| {
                    (
                        name.clone(),
                        CompiledVolume {
                            durability: vol.durability,
                            backup: vol.backup,
                        },
                    )
                })
                .collect(),
            window: CompiledWindow {
                title: manifest.window.title.clone(),
                width: manifest.window.width,
                height: manifest.window.height,
                entry: manifest.window.entry.clone(),
            },
            lifecycle: CompiledLifecycle {
                on_window_open: manifest.lifecycle.on_window_open,
                on_window_close: manifest.lifecycle.on_window_close,
            },
            install: CompiledInstall {
                mode: manifest.install.mode,
                ask_questions: manifest.install.ask_questions,
            },
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
