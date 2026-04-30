use std::fs;
use std::path::{Path, PathBuf};

use crate::manifest::compiled_manifest::CompiledManifest;

pub struct BundleReader {
    bundle_dir: PathBuf,
}

impl BundleReader {
    pub fn new(bundle_dir: &Path) -> Self {
        Self {
            bundle_dir: bundle_dir.to_path_buf(),
        }
    }

    pub fn read_manifest(&self) -> Result<CompiledManifest, bento_core::BentoError> {
        let manifest_path = self.bundle_dir.join("manifest.json");
        let content = fs::read_to_string(&manifest_path).map_err(|e| {
            bento_core::BentoError::BundleReadError(format!(
                "failed to read manifest.json at {}: {}",
                manifest_path.display(),
                e
            ))
        })?;
        serde_json::from_str(&content).map_err(|e| {
            bento_core::BentoError::BundleReadError(format!(
                "failed to parse manifest.json: {}",
                e
            ))
        })
    }

    pub fn image_path(&self, archive_name: &str) -> PathBuf {
        self.bundle_dir.join(archive_name)
    }

    pub fn bundle_dir(&self) -> &Path {
        &self.bundle_dir
    }

    pub fn validate_structure(&self) -> Result<(), bento_core::BentoError> {
        let required = ["manifest.json", "images"];
        for entry in required {
            let path = self.bundle_dir.join(entry);
            if !path.exists() {
                return Err(bento_core::BentoError::BundleReadError(format!(
                    "missing required bundle entry: {}",
                    entry
                )));
            }
        }
        Ok(())
    }
}
