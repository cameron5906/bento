use std::path::{Path, PathBuf};

use clap::Args;

use crate::output;

#[derive(Args)]
pub struct InitArgs {
    /// Path to existing docker-compose.yml
    #[arg(short, long, default_value = "./docker-compose.yml")]
    pub compose: PathBuf,

    /// Output path for craterun.yml
    #[arg(short, long, default_value = "./craterun.yml")]
    pub output: PathBuf,
}

pub async fn run(args: InitArgs) -> anyhow::Result<()> {
    output::header("CrateRun Init");

    if !args.compose.exists() {
        anyhow::bail!(
            "docker-compose.yml not found at {}",
            args.compose.display()
        );
    }

    if args.output.exists() {
        anyhow::bail!(
            "craterun.yml already exists at {}. Remove it first.",
            args.output.display()
        );
    }

    let compose = craterun_bundle::compose::ComposeFile::from_file(&args.compose)?;

    let services: Vec<&str> = compose.services.keys().map(|s| s.as_str()).collect();
    output::info(&format!("Found services: {}", services.join(", ")));

    let first_service = services.first().copied().unwrap_or("web");
    let first_port = compose
        .services
        .values()
        .next()
        .and_then(|s| s.ports.first())
        .and_then(|p| p.container_port())
        .unwrap_or(3000);

    let template = generate_template(first_service, first_port, &args.compose);
    std::fs::write(&args.output, template)?;

    output::success(&format!("Created {}", args.output.display()));
    output::info("Edit craterun.yml to configure your app metadata, routes, and health checks.");

    Ok(())
}

fn generate_template(frontend_service: &str, frontend_port: u16, compose_path: &Path) -> String {
    format!(
        r#"app:
  id: com.example.myapp
  name: My App
  version: 0.1.0
  icon: ./assets/icon.png

compose:
  file: {}
  projectName: myapp

window:
  title: My App
  width: 1200
  height: 800
  entry: /

routes:
  /:
    service: {}
    port: {}

health:
  ready:
    service: {}
    path: /health
    timeoutSeconds: 120

volumes: {{}}

lifecycle:
  onWindowOpen: startServices
  onWindowClose: stopServices

install:
  mode: consumer
  askQuestions: false
"#,
        compose_path.display(),
        frontend_service,
        frontend_port,
        frontend_service,
    )
}
