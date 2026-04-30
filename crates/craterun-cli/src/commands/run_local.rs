use std::path::PathBuf;

use clap::Args;

use crate::output;

#[derive(Args)]
pub struct RunLocalArgs {
    /// Path to craterun.yml
    #[arg(short, long, default_value = "./craterun.yml")]
    pub manifest: PathBuf,
}

pub async fn run(args: RunLocalArgs) -> anyhow::Result<()> {
    output::header("CrateRun Run Local");

    output::info("Building app bundle...");
    let build_args = super::build::BuildArgs {
        manifest: args.manifest,
        output: PathBuf::from("./dist/bundle"),
        target: "local".into(),
    };
    super::build::run(build_args).await?;

    output::info("Local supervisor launch not yet implemented.");
    output::info("Once implemented, this will:");
    output::info("  1. Spawn the supervisor with the built bundle");
    output::info("  2. Wait for app readiness");
    output::info("  3. Print the local app URL");

    Ok(())
}
