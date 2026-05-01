use clap::{Parser, Subcommand};

use crate::commands::{bento_box, build, certify, init, package, run_local};

#[derive(Parser)]
#[command(name = "bento")]
#[command(about = "Turn Docker Compose apps into consumer desktop installers")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new bento.yml from an existing docker-compose.yml
    Init(init::InitArgs),

    /// Check developer machine prerequisites
    Doctor,

    /// Build images and produce an app bundle
    Build(build::BuildArgs),

    /// Check if an app is safe for consumer packaging
    Certify(certify::CertifyArgs),

    /// Package app into a consumer desktop installer
    #[command(name = "box")]
    Box(bento_box::BoxArgs),

    /// Package app (use 'bento box' instead)
    #[command(hide = true)]
    Package(package::PackageArgs),

    /// Build and run the app locally
    #[command(name = "run-local")]
    RunLocal(run_local::RunLocalArgs),
}
