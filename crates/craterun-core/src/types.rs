use serde::{Deserialize, Serialize};

use crate::error::UserFacingError;
use crate::state::SupervisorState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub app_id: String,
    pub state: SupervisorState,
    pub message: String,
    pub progress: f32,
    pub app_url: Option<String>,
    pub error: Option<UserFacingError>,
    pub services: Vec<ServiceStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    pub name: String,
    pub state: ContainerState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContainerState {
    Pending,
    Starting,
    Running,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub timestamp: String,
    pub service: String,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupervisorSockInfo {
    pub port: u16,
    pub token: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VolumeDurability {
    Persistent,
    Disposable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ServiceRole {
    Frontend,
    Backend,
    Worker,
    Database,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RestartPolicy {
    No,
    Always,
    OnFailure,
    UnlessStopped,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self::No
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InstallMode {
    Consumer,
    DevPack,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LifecycleAction {
    StartServices,
    StopServices,
    Nothing,
}
