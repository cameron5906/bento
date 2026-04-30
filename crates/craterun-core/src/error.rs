use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum CrateRunError {
    #[error("bundle read error: {0}")]
    BundleReadError(String),

    #[error("manifest parse error: {0}")]
    ManifestParseError(String),

    #[error("compose validation failed: {0}")]
    ComposeValidationError(String),

    #[error("runtime not found: {0}")]
    RuntimeNotFound(String),

    #[error("image import failed for service '{service}': {detail}")]
    ImageImportFailed { service: String, detail: String },

    #[error("container start failed for service '{service}': {detail}")]
    ContainerStartFailed { service: String, detail: String },

    #[error("health check timeout for service '{service}' after {elapsed_seconds}s")]
    HealthCheckTimeout {
        service: String,
        elapsed_seconds: u32,
    },

    #[error("proxy bind failed on port {port}")]
    ProxyBindFailed { port: u16 },

    #[error("low disk space: need {required_bytes} bytes, have {available_bytes}")]
    DiskSpaceLow {
        required_bytes: u64,
        available_bytes: u64,
    },

    #[error("WSL not available: {0}")]
    WslNotAvailable(String),

    #[error("WSL feature requires reboot")]
    WslFeatureRequiresReboot,

    #[error("virtualization is disabled")]
    VirtualizationDisabled,

    #[error("container runtime error: {0}")]
    RuntimeError(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    LowDiskSpace,
    HealthTimeout,
    RuntimeNotFound,
    WslNotAvailable,
    WslRequiresReboot,
    VirtualizationDisabled,
    ImageImportFailed,
    ContainerStartFailed,
    ProxyBindFailed,
    BundleCorrupted,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorSeverity {
    Recoverable,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserAction {
    Retry,
    Repair,
    ExportDiagnostics,
    ResetData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFacingError {
    pub code: ErrorCode,
    pub severity: ErrorSeverity,
    pub user_title: String,
    pub user_message: String,
    pub technical_message: String,
    pub actions: Vec<UserAction>,
}

impl From<&CrateRunError> for UserFacingError {
    fn from(err: &CrateRunError) -> Self {
        match err {
            CrateRunError::DiskSpaceLow {
                required_bytes,
                available_bytes,
            } => {
                let required_gb = *required_bytes as f64 / 1_073_741_824.0;
                UserFacingError {
                    code: ErrorCode::LowDiskSpace,
                    severity: ErrorSeverity::Recoverable,
                    user_title: "Your computer is low on storage".into(),
                    user_message: format!(
                        "The app needs about {:.1} GB free to finish setup.",
                        required_gb
                    ),
                    technical_message: format!(
                        "required {} bytes, available {} bytes",
                        required_bytes, available_bytes
                    ),
                    actions: vec![UserAction::Retry, UserAction::ExportDiagnostics],
                }
            }

            CrateRunError::HealthCheckTimeout {
                service,
                elapsed_seconds,
            } => UserFacingError {
                code: ErrorCode::HealthTimeout,
                severity: ErrorSeverity::Recoverable,
                user_title: "The app is taking longer than expected to start".into(),
                user_message: "You can try again or repair the app.".into(),
                technical_message: format!(
                    "health check for '{}' failed after {}s",
                    service, elapsed_seconds
                ),
                actions: vec![UserAction::Retry, UserAction::Repair, UserAction::ExportDiagnostics],
            },

            CrateRunError::WslNotAvailable(detail) => UserFacingError {
                code: ErrorCode::WslNotAvailable,
                severity: ErrorSeverity::Blocked,
                user_title: "Required Windows feature is not available".into(),
                user_message: "This app requires a Windows feature that is not enabled on your computer.".into(),
                technical_message: detail.clone(),
                actions: vec![UserAction::ExportDiagnostics],
            },

            CrateRunError::WslFeatureRequiresReboot => UserFacingError {
                code: ErrorCode::WslRequiresReboot,
                severity: ErrorSeverity::Blocked,
                user_title: "Windows needs to restart".into(),
                user_message: "Windows needs to restart to finish setting up local app services.".into(),
                technical_message: "WSL feature installation requires system reboot".into(),
                actions: vec![],
            },

            CrateRunError::VirtualizationDisabled => UserFacingError {
                code: ErrorCode::VirtualizationDisabled,
                severity: ErrorSeverity::Blocked,
                user_title: "Hardware virtualization is disabled".into(),
                user_message: "This app requires hardware virtualization, which is currently disabled in your computer's BIOS settings.".into(),
                technical_message: "Hyper-V / hardware virtualization not detected".into(),
                actions: vec![UserAction::ExportDiagnostics],
            },

            CrateRunError::ImageImportFailed { service, detail } => UserFacingError {
                code: ErrorCode::ImageImportFailed,
                severity: ErrorSeverity::Recoverable,
                user_title: "The app could not finish setup".into(),
                user_message: "The app package may be damaged. Try repairing.".into(),
                technical_message: format!("image import for '{}': {}", service, detail),
                actions: vec![UserAction::Repair, UserAction::ExportDiagnostics],
            },

            CrateRunError::ContainerStartFailed { service, detail } => UserFacingError {
                code: ErrorCode::ContainerStartFailed,
                severity: ErrorSeverity::Recoverable,
                user_title: "The app had trouble starting".into(),
                user_message: "Try restarting the app or repairing it.".into(),
                technical_message: format!("container '{}' failed: {}", service, detail),
                actions: vec![UserAction::Retry, UserAction::Repair, UserAction::ExportDiagnostics],
            },

            CrateRunError::ProxyBindFailed { port } => UserFacingError {
                code: ErrorCode::ProxyBindFailed,
                severity: ErrorSeverity::Recoverable,
                user_title: "Could not start the app".into(),
                user_message: "A network resource needed by the app is unavailable. Try again.".into(),
                technical_message: format!("failed to bind proxy on port {}", port),
                actions: vec![UserAction::Retry, UserAction::ExportDiagnostics],
            },

            CrateRunError::RuntimeNotFound(detail) => UserFacingError {
                code: ErrorCode::RuntimeNotFound,
                severity: ErrorSeverity::Blocked,
                user_title: "Required runtime not found".into(),
                user_message: "A required component is missing. The app cannot run on this computer.".into(),
                technical_message: detail.clone(),
                actions: vec![UserAction::ExportDiagnostics],
            },

            CrateRunError::BundleReadError(detail) => UserFacingError {
                code: ErrorCode::BundleCorrupted,
                severity: ErrorSeverity::Recoverable,
                user_title: "App package is damaged".into(),
                user_message: "The app installation may be corrupted. Try reinstalling.".into(),
                technical_message: detail.clone(),
                actions: vec![UserAction::ExportDiagnostics],
            },

            _ => UserFacingError {
                code: ErrorCode::Unknown,
                severity: ErrorSeverity::Recoverable,
                user_title: "Something went wrong".into(),
                user_message: "An unexpected error occurred. Try restarting the app.".into(),
                technical_message: err.to_string(),
                actions: vec![UserAction::Retry, UserAction::ExportDiagnostics],
            },
        }
    }
}
