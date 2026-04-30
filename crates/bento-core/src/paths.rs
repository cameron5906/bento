use std::path::PathBuf;

use crate::AppId;

pub struct AppPaths {
    app_id: AppId,
    app_name: String,
}

impl AppPaths {
    pub fn new(app_id: AppId, app_name: String) -> Self {
        Self { app_id, app_name }
    }

    /// Where the app binaries and bundle are installed.
    /// - Windows: %LOCALAPPDATA%\Programs\<AppName>
    /// - macOS:   ~/Applications/<AppName>
    /// - Linux:   ~/.local/share/Programs/<AppName>
    pub fn install_dir(&self) -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("Applications")
                .join(&self.app_name)
        }
        #[cfg(not(target_os = "macos"))]
        {
            data_local_dir()
                .join("Programs")
                .join(&self.app_name)
        }
    }

    /// Where app runtime state, volumes, logs, and config live.
    /// - Windows: %LOCALAPPDATA%\Bento\Apps\<appId>
    /// - macOS:   ~/Library/Application Support/Bento/Apps/<appId>
    /// - Linux:   ~/.local/share/Bento/Apps/<appId>
    pub fn data_dir(&self) -> PathBuf {
        data_local_dir()
            .join("Bento")
            .join("Apps")
            .join(self.app_id.as_str())
    }

    pub fn bundle_path(&self) -> PathBuf {
        self.install_dir().join("bundle")
    }

    pub fn runtime_dir(&self) -> PathBuf {
        self.data_dir().join("runtime")
    }

    pub fn volumes_dir(&self) -> PathBuf {
        self.data_dir().join("volumes")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir().join("logs")
    }

    pub fn config_dir(&self) -> PathBuf {
        self.data_dir().join("config")
    }

    pub fn state_file(&self) -> PathBuf {
        self.config_dir().join("state.json")
    }

    pub fn supervisor_sock_file(&self) -> PathBuf {
        self.config_dir().join("supervisor.sock.json")
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.data_dir().join("backups")
    }
}

/// Platform-appropriate local data directory via the `dirs` crate.
/// Falls back to reasonable defaults if the directory cannot be determined.
fn data_local_dir() -> PathBuf {
    dirs::data_local_dir().unwrap_or_else(|| {
        #[cfg(windows)]
        {
            PathBuf::from(
                std::env::var("LOCALAPPDATA")
                    .unwrap_or_else(|_| "C:\\Users\\Default\\AppData\\Local".into()),
            )
        }
        #[cfg(not(windows))]
        {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join(".local")
                .join("share")
        }
    })
}
