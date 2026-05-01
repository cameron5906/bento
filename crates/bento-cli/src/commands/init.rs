use std::path::{Path, PathBuf};

use clap::Args;

use crate::output;

/// Standard compose filenames in priority order
const COMPOSE_FILENAMES: &[&str] = &[
    "docker-compose.yml",
    "docker-compose.yaml",
    "compose.yml",
    "compose.yaml",
];

#[derive(Args)]
pub struct InitArgs {
    /// Path to existing docker-compose file (auto-detected if not specified)
    #[arg(short, long)]
    pub compose: Option<PathBuf>,

    /// Output path for bento.yml
    #[arg(short, long, default_value = "./bento.yml")]
    pub output: PathBuf,
}

pub async fn run(args: InitArgs) -> anyhow::Result<()> {
    output::header("Bento Init");

    let compose_path = match args.compose {
        Some(p) => {
            if !p.exists() {
                anyhow::bail!("Compose file not found at {}", p.display());
            }
            p
        }
        None => find_compose_file()?,
    };

    output::success(&format!("Found compose file: {}", compose_path.display()));

    if args.output.exists() {
        anyhow::bail!(
            "bento.yml already exists at {}. Remove it first.",
            args.output.display()
        );
    }

    let compose = bento_bundle::compose::ComposeFile::from_file(&compose_path)?;

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

    let template = generate_template(first_service, first_port, &compose_path);
    std::fs::write(&args.output, template)?;

    output::success(&format!("Created {}", args.output.display()));
    output::info("Edit bento.yml to configure your app metadata, routes, and health checks.");
    output::info(&format!("Then run: bento box --target {}", crate::platform::default_build_target()));

    Ok(())
}

fn find_compose_file() -> anyhow::Result<PathBuf> {
    for name in COMPOSE_FILENAMES {
        let path = PathBuf::from(name);
        if path.exists() {
            return Ok(path);
        }
    }
    anyhow::bail!(
        "No compose file found in current directory.\n\
         Looked for: {}\n\
         Use --compose to specify the path.",
        COMPOSE_FILENAMES.join(", ")
    )
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
