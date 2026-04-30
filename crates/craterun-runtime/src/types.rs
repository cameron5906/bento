use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use craterun_core::types::{ContainerState, RestartPolicy};
use craterun_core::AppId;

#[derive(Debug, Clone)]
pub struct RuntimeDetectionResult {
    pub available: bool,
    pub version: Option<String>,
    pub requires_preparation: bool,
    pub blocker: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimePlan {
    pub app_id: AppId,
    pub services: Vec<PlannedService>,
    pub network_name: String,
}

#[derive(Debug, Clone)]
pub struct PlannedService {
    pub name: String,
    pub image_ref: ImageRef,
    pub container_port: u16,
    pub host_port: u16,
    pub env: IndexMap<String, String>,
    pub depends_on: Vec<String>,
    pub volumes: Vec<VolumeMount>,
    pub restart_policy: RestartPolicy,
}

#[derive(Debug, Clone)]
pub struct ImageRef {
    pub name: String,
    pub digest: Option<String>,
    pub archive_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VolumeMount {
    pub volume_name: String,
    pub container_path: String,
}

#[derive(Debug, Clone)]
pub struct RemoveOptions {
    pub remove_volumes: bool,
    pub remove_runtime: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStatus {
    pub name: String,
    pub state: ContainerState,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct LogLine {
    pub timestamp: String,
    pub service: String,
    pub message: String,
}
