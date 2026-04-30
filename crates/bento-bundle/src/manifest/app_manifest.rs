use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use bento_core::types::{InstallMode, LifecycleAction, VolumeDurability};
use bento_core::AppId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppManifest {
    pub app: AppConfig,
    pub compose: ComposeConfig,
    pub window: WindowConfig,
    pub routes: IndexMap<String, RouteTarget>,
    pub health: HealthConfig,
    #[serde(default)]
    pub volumes: IndexMap<String, VolumeConfig>,
    #[serde(default)]
    pub lifecycle: LifecycleConfig,
    #[serde(default)]
    pub install: InstallConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub id: AppId,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub icon: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeConfig {
    pub file: PathBuf,
    #[serde(default)]
    pub project_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub title: String,
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
    #[serde(default = "default_entry")]
    pub entry: String,
}

fn default_width() -> u32 {
    1200
}
fn default_height() -> u32 {
    800
}
fn default_entry() -> String {
    "/".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteTarget {
    pub service: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    pub ready: HealthCheck,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheck {
    pub service: String,
    pub path: String,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u32,
    #[serde(default = "default_interval")]
    pub interval_seconds: u32,
}

fn default_timeout() -> u32 {
    120
}
fn default_interval() -> u32 {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeConfig {
    #[serde(default = "default_durability")]
    pub durability: VolumeDurability,
    #[serde(default)]
    pub backup: bool,
}

fn default_durability() -> VolumeDurability {
    VolumeDurability::Persistent
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleConfig {
    #[serde(default = "default_on_open")]
    pub on_window_open: LifecycleAction,
    #[serde(default = "default_on_close")]
    pub on_window_close: LifecycleAction,
}

fn default_on_open() -> LifecycleAction {
    LifecycleAction::StartServices
}
fn default_on_close() -> LifecycleAction {
    LifecycleAction::StopServices
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            on_window_open: default_on_open(),
            on_window_close: default_on_close(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallConfig {
    #[serde(default = "default_install_mode")]
    pub mode: InstallMode,
    #[serde(default)]
    pub ask_questions: bool,
}

fn default_install_mode() -> InstallMode {
    InstallMode::Consumer
}

impl Default for InstallConfig {
    fn default() -> Self {
        Self {
            mode: default_install_mode(),
            ask_questions: false,
        }
    }
}

impl AppManifest {
    pub fn from_file(path: &Path) -> Result<Self, bento_core::BentoError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            bento_core::BentoError::ManifestParseError(format!(
                "failed to read {}: {}",
                path.display(),
                e
            ))
        })?;
        Self::from_str(&content)
    }

    pub fn from_str(yaml: &str) -> Result<Self, bento_core::BentoError> {
        serde_yaml::from_str(yaml).map_err(|e| {
            bento_core::BentoError::ManifestParseError(format!("invalid bento.yml: {}", e))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_MANIFEST: &str = r#"
app:
  id: com.example.photobooth
  name: PhotoBooth Local
  version: 1.0.0
  icon: ./assets/icon.png

compose:
  file: ./docker-compose.yml
  projectName: photobooth

window:
  title: PhotoBooth Local
  width: 1200
  height: 800
  entry: /

routes:
  /:
    service: web
    port: 3000
  /api:
    service: api
    port: 8080

health:
  ready:
    service: api
    path: /health
    timeoutSeconds: 120

volumes:
  db-data:
    durability: persistent
    backup: true

lifecycle:
  onWindowOpen: startServices
  onWindowClose: stopServices

install:
  mode: consumer
  askQuestions: false
"#;

    #[test]
    fn parse_sample_manifest() {
        let manifest = AppManifest::from_str(SAMPLE_MANIFEST).unwrap();
        assert_eq!(manifest.app.name, "PhotoBooth Local");
        assert_eq!(manifest.app.id.as_str(), "com.example.photobooth");
        assert_eq!(manifest.routes.len(), 2);
        assert_eq!(manifest.health.ready.service, "api");
        assert_eq!(manifest.health.ready.path, "/health");
        assert_eq!(manifest.volumes.len(), 1);
        assert_eq!(
            manifest.volumes["db-data"].durability,
            VolumeDurability::Persistent
        );
    }
}
