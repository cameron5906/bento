use std::path::PathBuf;

use clap::Args;

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
}

pub async fn run(args: PackageArgs) -> anyhow::Result<()> {
    output::header("CrateRun Package");

    if args.consumer {
        output::info("Running consumer certification checks...");
        let certify_args = super::certify::CertifyArgs {
            manifest: args.manifest.clone(),
        };
        super::certify::run(certify_args).await?;

        output::info("Running build...");
        let build_args = super::build::BuildArgs {
            manifest: args.manifest,
            output: args.output.join("bundle"),
            target: args.target,
        };
        super::build::run(build_args).await?;

        output::info("Installer generation not yet implemented.");
        output::success("Consumer package preparation complete (bundle only).");
    } else {
        output::info("Running build...");
        let build_args = super::build::BuildArgs {
            manifest: args.manifest,
            output: args.output.join("bundle"),
            target: args.target,
        };
        super::build::run(build_args).await?;
    }

    Ok(())
}
