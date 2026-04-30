mod cli;
mod commands;
mod installer;
mod output;

use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init(args) => commands::init::run(args).await?,
        Commands::Doctor => commands::doctor::run().await?,
        Commands::Build(args) => commands::build::run(args).await?,
        Commands::Certify(args) => commands::certify::run(args).await?,
        Commands::Package(args) => commands::package::run(args).await?,
        Commands::RunLocal(args) => commands::run_local::run(args).await?,
    }

    Ok(())
}
