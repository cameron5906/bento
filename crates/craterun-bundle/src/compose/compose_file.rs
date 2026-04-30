use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeFile {
    #[serde(default)]
    pub version: Option<String>,
    pub services: IndexMap<String, ComposeService>,
    #[serde(default)]
    pub volumes: IndexMap<String, Option<ComposeVolumeConfig>>,
    #[serde(default)]
    pub networks: IndexMap<String, Option<ComposeNetworkConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeService {
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub build: Option<ComposeBuild>,
    #[serde(default)]
    pub ports: Vec<ComposePort>,
    #[serde(default)]
    pub environment: Option<ComposeEnvironment>,
    #[serde(default)]
    pub volumes: Vec<String>,
    #[serde(default)]
    pub depends_on: Option<ComposeDependsOn>,
    #[serde(default)]
    pub restart: Option<String>,
    #[serde(default)]
    pub healthcheck: Option<ComposeHealthCheck>,
    #[serde(default)]
    pub privileged: Option<bool>,
    #[serde(default)]
    pub network_mode: Option<String>,
    #[serde(default)]
    pub pid: Option<String>,
    #[serde(default)]
    pub ipc: Option<String>,
    #[serde(default)]
    pub cap_add: Vec<String>,
    #[serde(default)]
    pub networks: Option<serde_yaml::Value>,
    #[serde(default)]
    pub command: Option<serde_yaml::Value>,
    #[serde(default)]
    pub entrypoint: Option<serde_yaml::Value>,
    #[serde(default)]
    pub working_dir: Option<String>,
    #[serde(default)]
    pub container_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ComposeBuild {
    Simple(String),
    Detailed(ComposeBuildConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeBuildConfig {
    pub context: String,
    #[serde(default)]
    pub dockerfile: Option<String>,
    #[serde(default)]
    pub args: Option<IndexMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ComposePort {
    Short(String),
    Long(ComposePortLong),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposePortLong {
    pub target: u16,
    #[serde(default)]
    pub published: Option<String>,
    #[serde(default)]
    pub protocol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ComposeEnvironment {
    Map(IndexMap<String, serde_yaml::Value>),
    List(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ComposeDependsOn {
    Simple(Vec<String>),
    Detailed(IndexMap<String, ComposeDependsOnCondition>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeDependsOnCondition {
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeHealthCheck {
    #[serde(default)]
    pub test: Option<serde_yaml::Value>,
    #[serde(default)]
    pub interval: Option<String>,
    #[serde(default)]
    pub timeout: Option<String>,
    #[serde(default)]
    pub retries: Option<u32>,
    #[serde(default)]
    pub start_period: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeVolumeConfig {
    #[serde(default)]
    pub external: Option<bool>,
    #[serde(default)]
    pub driver: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeNetworkConfig {
    #[serde(default)]
    pub external: Option<bool>,
    #[serde(default)]
    pub driver: Option<String>,
}

impl ComposeFile {
    pub fn from_file(path: &Path) -> Result<Self, craterun_core::CrateRunError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            craterun_core::CrateRunError::ManifestParseError(format!(
                "failed to read {}: {}",
                path.display(),
                e
            ))
        })?;
        Self::from_str(&content)
    }

    pub fn from_str(yaml: &str) -> Result<Self, craterun_core::CrateRunError> {
        serde_yaml::from_str(yaml).map_err(|e| {
            craterun_core::CrateRunError::ManifestParseError(format!(
                "invalid docker-compose.yml: {}",
                e
            ))
        })
    }

    pub fn service_names(&self) -> Vec<String> {
        self.services.keys().cloned().collect()
    }

    pub fn depends_on_list(&self, service: &str) -> Vec<String> {
        self.services
            .get(service)
            .and_then(|s| s.depends_on.as_ref())
            .map(|d| match d {
                ComposeDependsOn::Simple(list) => list.clone(),
                ComposeDependsOn::Detailed(map) => map.keys().cloned().collect(),
            })
            .unwrap_or_default()
    }
}

impl ComposePort {
    pub fn has_fixed_host_port(&self) -> Option<u16> {
        match self {
            ComposePort::Short(s) => {
                let parts: Vec<&str> = s.split(':').collect();
                if parts.len() >= 2 {
                    parts[0].parse::<u16>().ok()
                } else {
                    None
                }
            }
            ComposePort::Long(l) => l
                .published
                .as_ref()
                .and_then(|p| p.parse::<u16>().ok()),
        }
    }

    pub fn container_port(&self) -> Option<u16> {
        match self {
            ComposePort::Short(s) => {
                let parts: Vec<&str> = s.split(':').collect();
                parts.last().and_then(|p| p.parse::<u16>().ok())
            }
            ComposePort::Long(l) => Some(l.target),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_COMPOSE: &str = r#"
services:
  web:
    image: myapp-web
    ports:
      - "3000"
  api:
    image: myapp-api
    ports:
      - "8080"
    environment:
      DATABASE_URL: postgres://postgres:postgres@db:5432/app
    depends_on:
      - db
  db:
    image: postgres:16
    volumes:
      - db-data:/var/lib/postgresql/data

volumes:
  db-data:
"#;

    #[test]
    fn parse_sample_compose() {
        let compose = ComposeFile::from_str(SAMPLE_COMPOSE).unwrap();
        assert_eq!(compose.services.len(), 3);
        assert!(compose.services.contains_key("web"));
        assert!(compose.services.contains_key("api"));
        assert!(compose.services.contains_key("db"));
        assert_eq!(compose.volumes.len(), 1);
    }

    #[test]
    fn depends_on_list() {
        let compose = ComposeFile::from_str(SAMPLE_COMPOSE).unwrap();
        assert_eq!(compose.depends_on_list("api"), vec!["db"]);
        assert!(compose.depends_on_list("web").is_empty());
    }

    #[test]
    fn container_port_parsing() {
        let port = ComposePort::Short("3000".into());
        assert_eq!(port.container_port(), Some(3000));
        assert_eq!(port.has_fixed_host_port(), None);

        let port_fixed = ComposePort::Short("8080:3000".into());
        assert_eq!(port_fixed.container_port(), Some(3000));
        assert_eq!(port_fixed.has_fixed_host_port(), Some(8080));
    }
}
