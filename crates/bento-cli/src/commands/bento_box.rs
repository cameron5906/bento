use std::path::PathBuf;

use clap::Args;

use crate::output;

/// Package a Docker Compose app into a consumer desktop installer.
/// This is the main developer-facing command — the final step that
/// produces an installable .exe, .dmg, or .deb/.AppImage.
#[derive(Args)]
pub struct BoxArgs {
    /// Target platform (auto-detected from current OS)
    #[arg(long, default_value_t = crate::platform::default_build_target())]
    pub target: String,

    /// Path to bento.yml
    #[arg(short, long, default_value = "./bento.yml")]
    pub manifest: PathBuf,

    /// Output directory
    #[arg(short, long, default_value = "./dist")]
    pub output: PathBuf,

    /// Skip installer compilation (only generate the script/stage files)
    #[arg(long)]
    pub script_only: bool,
}

pub async fn run(args: BoxArgs) -> anyhow::Result<()> {
    output::header("Bento Box");

    let package_args = super::package::PackageArgs {
        consumer: true,
        target: args.target,
        manifest: args.manifest,
        output: args.output,
        script_only: args.script_only,
    };

    super::package::run(package_args).await
}
