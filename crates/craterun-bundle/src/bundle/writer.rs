use std::fs;
use std::path::{Path, PathBuf};

use crate::manifest::compiled_manifest::CompiledManifest;

pub struct BundleWriter {
    output_dir: PathBuf,
}

impl BundleWriter {
    pub fn new(output_dir: &Path) -> Self {
        Self {
            output_dir: output_dir.to_path_buf(),
        }
    }

    pub fn write_manifest(&self, manifest: &CompiledManifest) -> Result<(), craterun_core::CrateRunError> {
        let manifest_path = self.output_dir.join("manifest.json");
        let json = manifest.to_json().map_err(|e| {
            craterun_core::CrateRunError::BundleReadError(format!(
                "failed to serialize manifest: {}",
                e
            ))
        })?;
        fs::create_dir_all(&self.output_dir).map_err(craterun_core::CrateRunError::Io)?;
        fs::write(&manifest_path, json).map_err(craterun_core::CrateRunError::Io)?;
        Ok(())
    }

    pub fn ensure_dirs(&self) -> Result<(), craterun_core::CrateRunError> {
        let dirs = ["images", "assets", "shell", "sbom", "signatures"];
        for dir in dirs {
            fs::create_dir_all(self.output_dir.join(dir))
                .map_err(craterun_core::CrateRunError::Io)?;
        }
        Ok(())
    }

    pub fn write_shell_config(
        &self,
        app_id: &str,
        app_name: &str,
        window_title: &str,
        width: u32,
        height: u32,
    ) -> Result<(), craterun_core::CrateRunError> {
        let config = serde_json::json!({
            "appId": app_id,
            "appName": app_name,
            "windowTitle": window_title,
            "width": width,
            "height": height
        });
        let path = self.output_dir.join("shell").join("shell-config.json");
        let json = serde_json::to_string_pretty(&config).map_err(|e| {
            craterun_core::CrateRunError::BundleReadError(format!("json error: {}", e))
        })?;
        fs::write(&path, json).map_err(craterun_core::CrateRunError::Io)?;
        Ok(())
    }

    pub fn copy_asset(&self, src: &Path, filename: &str) -> Result<(), craterun_core::CrateRunError> {
        let dest = self.output_dir.join("assets").join(filename);
        fs::copy(src, &dest).map_err(craterun_core::CrateRunError::Io)?;
        Ok(())
    }

    pub fn images_dir(&self) -> PathBuf {
        self.output_dir.join("images")
    }

    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }
}
