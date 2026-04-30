use std::path::PathBuf;

use clap::Args;

use craterun_bundle::bundle::BundleReader;
use crate::installer::nsis::{InstallerBinaries, NsisInstaller};
use crate::output;

#[derive(Args)]
pub struct PackageArgs {
    /// Build a consumer installer
    #[arg(long)]
    pub consumer: bool,

    /// Target platform
    #[arg(long, default_value = "windows-x64")]
    pub target: String,

    /// Path to craterun.yml
    #[arg(short, long, default_value = "./craterun.yml")]
    pub manifest: PathBuf,

    /// Output directory
    #[arg(short, long, default_value = "./dist")]
    pub output: PathBuf,

    /// Skip NSIS compilation (only generate the .nsi script)
    #[arg(long)]
    pub script_only: bool,
}

pub async fn run(args: PackageArgs) -> anyhow::Result<()> {
    output::header("CrateRun Package");

    if args.consumer {
        output::info("Running consumer certification checks...");
        let certify_args = super::certify::CertifyArgs {
            manifest: args.manifest.clone(),
        };
        super::certify::run(certify_args).await?;
    }

    output::info("Running build...");
    let bundle_dir = args.output.join("bundle");
    let build_args = super::build::BuildArgs {
        manifest: args.manifest.clone(),
        output: bundle_dir.clone(),
        target: args.target.clone(),
        skip_images: false,
    };
    super::build::run(build_args).await?;

    if args.consumer && args.target.starts_with("windows") {
        output::info("Generating Windows installer...");

        let reader = BundleReader::new(&bundle_dir);
        let manifest = reader.read_manifest()?;

        // Locate the built binaries from the workspace target directory.
        // In a real release workflow these would be pre-built and placed
        // in a known location. For development, we look in target/release
        // or target/debug relative to the workspace root.
        let binaries = locate_binaries()?;

        let installer = NsisInstaller::new(
            manifest,
            bundle_dir.clone(),
            args.output.clone(),
        );

        let script_path = installer.generate_script(&binaries)?;
        output::success(&format!("Generated NSIS script: {}", script_path.display()));

        if args.script_only {
            output::info("--script-only: skipping makensis compilation.");
        } else {
            match installer.compile(&script_path) {
                Ok(exe_path) => {
                    output::success(&format!("Installer: {}", exe_path.display()));
                }
                Err(e) => {
                    output::failure(&format!("{}", e));
                    output::info("The .nsi script was generated and can be compiled manually.");
                }
            }
        }
    } else if args.consumer {
        output::info("Installer generation is only supported for windows targets.");
    }

    output::success("Package complete.");
    Ok(())
}

/// Try to find the shell and supervisor binaries in the workspace target dir.
/// Falls back to placeholder paths if not found (the .nsi script is still
/// generated and can be edited before compilation).
fn locate_binaries() -> anyhow::Result<InstallerBinaries> {
    let shell_names = ["craterun-shell.exe", "craterun_shell.exe"];
    let supervisor_names = ["craterun-supervisor.exe", "craterun_supervisor.exe"];

    let mut shell_exe = PathBuf::from("craterun-shell.exe");
    let mut supervisor_exe = PathBuf::from("craterun-supervisor.exe");

    // Walk up from CWD to find the workspace root (contains target/)
    let mut search_roots = vec![PathBuf::from(".")];
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir = cwd.as_path();
        while let Some(parent) = dir.parent() {
            search_roots.push(parent.to_path_buf());
            dir = parent;
        }
    }

    let mut found_shell = false;
    let mut found_supervisor = false;

    // Search release first, then debug. Stop once both are found.
    'outer: for profile in &["release", "debug"] {
        for root in &search_roots {
            let target_dir = root.join("target").join(profile);
            if !target_dir.exists() {
                continue;
            }
            if !found_shell {
                for name in &shell_names {
                    let path = target_dir.join(name);
                    if path.exists() {
                        shell_exe = path;
                        found_shell = true;
                    }
                }
            }
            if !found_supervisor {
                for name in &supervisor_names {
                    let path = target_dir.join(name);
                    if path.exists() {
                        supervisor_exe = path;
                        found_supervisor = true;
                    }
                }
            }
            if found_shell && found_supervisor {
                break 'outer;
            }
        }
    }

    Ok(InstallerBinaries {
        shell_exe,
        supervisor_exe,
    })
}
