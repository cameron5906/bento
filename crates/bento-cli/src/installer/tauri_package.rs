//! macOS and Linux packaging via Tauri's native build system.
//!
//! Tauri produces .app + .dmg on macOS and .deb + .AppImage on Linux.
//! This module stages the app bundle into the Tauri project, invokes
//! `cargo tauri build`, and collects the output artifacts.
//!
//! No code signing is required for prototype distribution.
//! See docs/decisions.md ADR-008 for the Windows NSIS approach.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use bento_bundle::manifest::compiled_manifest::CompiledManifest;

use crate::output;

pub struct TauriPackager {
    #[allow(dead_code)] // used for future artifact naming
    manifest: CompiledManifest,
    bundle_dir: PathBuf,
    output_dir: PathBuf,
    workspace_root: PathBuf,
}

impl TauriPackager {
    pub fn new(
        manifest: CompiledManifest,
        bundle_dir: PathBuf,
        output_dir: PathBuf,
    ) -> anyhow::Result<Self> {
        let workspace_root = find_workspace_root()?;
        Ok(Self {
            manifest,
            bundle_dir,
            output_dir,
            workspace_root,
        })
    }

    /// Stage the app bundle into the Tauri project's resources directory
    /// so it gets embedded in the native app package.
    pub fn stage_bundle(&self) -> anyhow::Result<()> {
        let resources_dir = self.tauri_dir().join("resources").join("bundle");
        if resources_dir.exists() {
            std::fs::remove_dir_all(&resources_dir)?;
        }
        copy_dir_recursive(&self.bundle_dir, &resources_dir)?;
        output::success(&format!(
            "Staged bundle into {}",
            resources_dir.display()
        ));
        Ok(())
    }

    /// Stage the supervisor binary into Tauri resources.
    pub fn stage_supervisor(&self) -> anyhow::Result<()> {
        let supervisor_src = find_supervisor_binary(&self.workspace_root)?;
        let resources_dir = self.tauri_dir().join("resources");
        std::fs::create_dir_all(&resources_dir)?;

        #[cfg(windows)]
        let supervisor_name = "bento-supervisor.exe";
        #[cfg(not(windows))]
        let supervisor_name = "bento-supervisor";

        let dest = resources_dir.join(supervisor_name);
        std::fs::copy(&supervisor_src, &dest)?;
        output::success("Staged supervisor binary");
        Ok(())
    }

    /// Invoke `cargo tauri build` for the target platform.
    pub async fn build(&self, target: &str) -> anyhow::Result<Vec<PathBuf>> {
        let rust_target = match target {
            "macos-arm64" => "aarch64-apple-darwin",
            "macos-x64" => "x86_64-apple-darwin",
            "linux-x64" => "x86_64-unknown-linux-gnu",
            "linux-arm64" => "aarch64-unknown-linux-gnu",
            other => anyhow::bail!("unsupported Tauri build target: {}", other),
        };

        output::info(&format!(
            "Running cargo tauri build --target {} ...",
            rust_target
        ));

        let status = tokio::process::Command::new("cargo")
            .args(["tauri", "build", "--target", rust_target])
            .current_dir(self.tauri_project_dir())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;

        if !status.success() {
            anyhow::bail!("cargo tauri build failed");
        }

        let artifacts = self.collect_artifacts(rust_target)?;
        Ok(artifacts)
    }

    /// Run the full packaging pipeline: stage + build + collect.
    pub async fn run(&self, target: &str) -> anyhow::Result<Vec<PathBuf>> {
        self.stage_bundle()?;
        self.stage_supervisor()?;
        let artifacts = self.build(target).await?;

        std::fs::create_dir_all(&self.output_dir)?;
        let mut output_paths = Vec::new();
        for artifact in &artifacts {
            let filename = artifact.file_name().unwrap_or_default();
            let dest = self.output_dir.join(filename);
            std::fs::copy(artifact, &dest)?;
            output::success(&format!("Output: {}", dest.display()));
            output_paths.push(dest);
        }

        Ok(output_paths)
    }

    fn tauri_dir(&self) -> PathBuf {
        self.workspace_root
            .join("apps")
            .join("shell-tauri")
            .join("src-tauri")
    }

    fn tauri_project_dir(&self) -> PathBuf {
        self.workspace_root
            .join("apps")
            .join("shell-tauri")
    }

    /// Find Tauri build artifacts (.dmg, .deb, .AppImage) in the target directory.
    fn collect_artifacts(&self, rust_target: &str) -> anyhow::Result<Vec<PathBuf>> {
        let bundle_dir = self
            .tauri_dir()
            .join("target")
            .join(rust_target)
            .join("release")
            .join("bundle");

        // Also check the workspace-level target dir
        let alt_bundle_dir = self
            .workspace_root
            .join("target")
            .join(rust_target)
            .join("release")
            .join("bundle");

        let mut artifacts = Vec::new();

        for dir in [&bundle_dir, &alt_bundle_dir] {
            if !dir.exists() {
                continue;
            }
            // Walk known Tauri output subdirs
            for subdir in ["dmg", "deb", "appimage", "msi", "nsis"] {
                let sub = dir.join(subdir);
                if sub.exists() {
                    for entry in std::fs::read_dir(&sub)? {
                        let entry = entry?;
                        let path = entry.path();
                        if path.is_file() {
                            artifacts.push(path);
                        }
                    }
                }
            }
        }

        Ok(artifacts)
    }
}

/// Walk up from CWD to find the workspace root (directory containing Cargo.toml with [workspace]).
fn find_workspace_root() -> anyhow::Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = std::fs::read_to_string(&cargo_toml)?;
            if content.contains("[workspace]") {
                return Ok(dir);
            }
        }
        if !dir.pop() {
            anyhow::bail!(
                "could not find workspace root (Cargo.toml with [workspace]) \
                 above current directory"
            );
        }
    }
}

fn find_supervisor_binary(workspace_root: &Path) -> anyhow::Result<PathBuf> {
    #[cfg(windows)]
    let names = ["bento-supervisor.exe"];
    #[cfg(not(windows))]
    let names = ["bento-supervisor"];

    for profile in ["release", "debug"] {
        for name in &names {
            let path = workspace_root.join("target").join(profile).join(name);
            if path.exists() {
                return Ok(path);
            }
        }
    }

    anyhow::bail!(
        "supervisor binary not found in target/release or target/debug. \
         Run `cargo build --release -p bento-supervisor` first."
    )
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
