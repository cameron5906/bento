use std::fs;
use std::path::{Path, PathBuf};

use crate::manifest::compiled_manifest::CompiledManifest;

/// Built-in splash messages used when the app doesn't provide custom ones
pub fn default_splash_messages() -> Vec<String> {
    vec![
        "Unpacking your bento box...".into(),
        "Warming up the containers...".into(),
        "Preparing something delicious...".into(),
        "Almost ready to serve...".into(),
        "Arranging the compartments...".into(),
        "Fresh ingredients loading...".into(),
        "Plating your app...".into(),
        "Adding the finishing touches...".into(),
        "Your app is being boxed up...".into(),
        "Seasoning the services...".into(),
    ]
}

pub struct BundleWriter {
    output_dir: PathBuf,
}

impl BundleWriter {
    pub fn new(output_dir: &Path) -> Self {
        Self {
            output_dir: output_dir.to_path_buf(),
        }
    }

    pub fn write_manifest(&self, manifest: &CompiledManifest) -> Result<(), bento_core::BentoError> {
        let manifest_path = self.output_dir.join("manifest.json");
        let json = manifest.to_json().map_err(|e| {
            bento_core::BentoError::BundleReadError(format!(
                "failed to serialize manifest: {}",
                e
            ))
        })?;
        fs::create_dir_all(&self.output_dir).map_err(bento_core::BentoError::Io)?;
        fs::write(&manifest_path, json).map_err(bento_core::BentoError::Io)?;
        Ok(())
    }

    pub fn ensure_dirs(&self) -> Result<(), bento_core::BentoError> {
        let dirs = ["images", "assets", "shell", "sbom", "signatures"];
        for dir in dirs {
            fs::create_dir_all(self.output_dir.join(dir))
                .map_err(bento_core::BentoError::Io)?;
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
        splash_logo: Option<&str>,
        splash_messages: &[String],
    ) -> Result<(), bento_core::BentoError> {
        let config = serde_json::json!({
            "appId": app_id,
            "appName": app_name,
            "windowTitle": window_title,
            "width": width,
            "height": height,
            "splash": {
                "logo": splash_logo,
                "messages": if splash_messages.is_empty() { default_splash_messages() } else { splash_messages.to_vec() }
            }
        });
        let path = self.output_dir.join("shell").join("shell-config.json");
        let json = serde_json::to_string_pretty(&config).map_err(|e| {
            bento_core::BentoError::BundleReadError(format!("json error: {}", e))
        })?;
        fs::write(&path, json).map_err(bento_core::BentoError::Io)?;
        Ok(())
    }

    pub fn copy_asset(&self, src: &Path, filename: &str) -> Result<(), bento_core::BentoError> {
        let dest = self.output_dir.join("assets").join(filename);
        fs::copy(src, &dest).map_err(bento_core::BentoError::Io)?;
        Ok(())
    }

    pub fn images_dir(&self) -> PathBuf {
        self.output_dir.join("images")
    }

    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }
}
