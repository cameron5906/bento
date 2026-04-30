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

    pub fn install_dir(&self) -> PathBuf {
        local_app_data().join("Programs").join(&self.app_name)
    }

    pub fn data_dir(&self) -> PathBuf {
        local_app_data()
            .join("CrateRun")
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

fn local_app_data() -> PathBuf {
    std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs_fallback()
        })
}

fn dirs_fallback() -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from(std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".into()))
            .join("AppData")
            .join("Local")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
            .join(".local")
            .join("share")
    }
}
