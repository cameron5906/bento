use serde::{Deserialize, Serialize};

use crate::error::UserFacingError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum SupervisorState {
    NotInstalled,
    InstalledNotPrepared,
    CheckingSystem,
    PreparingRuntime,
    ImportingImages,
    CreatingNetwork,
    CreatingVolumes,
    StartingServices,
    StartingProxy,
    WaitingForHealth,
    Ready {
        app_url: String,
    },
    Stopping,
    Stopped,
    Repairing,
    FailedRecoverable {
        error: UserFacingError,
    },
    FailedBlocked {
        error: UserFacingError,
    },
    Uninstalling,
}

impl SupervisorState {
    pub fn user_message(&self, _app_name: &str) -> &'static str {
        match self {
            Self::NotInstalled => "Not installed",
            Self::InstalledNotPrepared => "Setting things up...",
            Self::CheckingSystem => "Checking your computer...",
            Self::PreparingRuntime => "Preparing local app services...",
            Self::ImportingImages => "Setting up the app...",
            Self::CreatingNetwork => "Setting up the app...",
            Self::CreatingVolumes => "Setting up the app...",
            Self::StartingServices => "Starting the app...",
            Self::StartingProxy => "Starting the app...",
            Self::WaitingForHealth => "Almost ready...",
            Self::Ready { .. } => "Ready",
            Self::Stopping => "Stopping...",
            Self::Stopped => "Stopped",
            Self::Repairing => "Repairing the app...",
            Self::FailedRecoverable { .. } => "The app had trouble starting.",
            Self::FailedBlocked { .. } => "The app cannot start on this computer yet.",
            Self::Uninstalling => "Uninstalling...",
        }
    }

    pub fn progress(&self) -> f32 {
        match self {
            Self::NotInstalled => 0.0,
            Self::InstalledNotPrepared => 0.0,
            Self::CheckingSystem => 0.05,
            Self::PreparingRuntime => 0.15,
            Self::ImportingImages => 0.35,
            Self::CreatingNetwork => 0.50,
            Self::CreatingVolumes => 0.55,
            Self::StartingServices => 0.65,
            Self::StartingProxy => 0.80,
            Self::WaitingForHealth => 0.90,
            Self::Ready { .. } => 1.0,
            Self::Stopping => 0.5,
            Self::Stopped => 0.0,
            Self::Repairing => 0.5,
            Self::FailedRecoverable { .. } => 0.0,
            Self::FailedBlocked { .. } => 0.0,
            Self::Uninstalling => 0.5,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Ready { .. }
                | Self::Stopped
                | Self::FailedRecoverable { .. }
                | Self::FailedBlocked { .. }
        )
    }
}
