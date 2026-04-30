use std::path::PathBuf;

use clap::Args;

use craterun_bundle::bundle::BundleWriter;
use craterun_bundle::compose::ComposeFile;
use craterun_bundle::manifest::compiled_manifest::{CompiledManifest, ServiceEntry};
use craterun_bundle::manifest::AppManifest;
use craterun_core::types::ServiceRole;

use crate::output;

#[derive(Args)]
pub struct BuildArgs {
    /// Path to craterun.yml
    #[arg(short, long, default_value = "./craterun.yml")]
    pub manifest: PathBuf,

    /// Output directory for the app bundle
    #[arg(short, long, default_value = "./dist/bundle")]
    pub output: PathBuf,

    /// Target platform
    #[arg(long, default_value = "windows-x64")]
    pub target: String,
}

pub async fn run(args: BuildArgs) -> anyhow::Result<()> {
    output::header("CrateRun Build");

    let manifest = AppManifest::from_file(&args.manifest)?;
    output::success(&format!("Loaded manifest: {}", manifest.app.name));

    let compose_path = args
        .manifest
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(&manifest.compose.file);

    let compose = ComposeFile::from_file(&compose_path)?;
    output::success(&format!(
        "Loaded compose file: {} services",
        compose.services.len()
    ));

    let services: Vec<ServiceEntry> = compose
        .services
        .iter()
        .map(|(name, svc)| {
            let port = svc
                .ports
                .first()
                .and_then(|p| p.container_port())
                .unwrap_or(0);

            let role = infer_role(name, svc);

            let _image_name = svc
                .image
                .clone()
                .unwrap_or_else(|| format!("{}-{}", manifest.app.id, name));

            ServiceEntry {
                name: name.clone(),
                image_archive: format!("images/{}-linux-amd64.oci.tar.zst", name),
                image_digest: None,
                container_port: port,
                role,
                env: extract_env(svc),
                depends_on: compose.depends_on_list(name),
                restart_policy: Default::default(),
            }
        })
        .collect();

    let compiled = CompiledManifest::from_app_manifest(&manifest, services, &args.target);

    let writer = BundleWriter::new(&args.output);
    writer.ensure_dirs()?;
    writer.write_manifest(&compiled)?;
    output::success("Wrote manifest.json");

    writer.write_shell_config(
        manifest.app.id.as_str(),
        &manifest.app.name,
        &manifest.window.title,
        manifest.window.width,
        manifest.window.height,
    )?;
    output::success("Wrote shell-config.json");

    output::info("Image build/export not yet implemented — bundle structure created.");
    output::success(&format!("Bundle output: {}", args.output.display()));

    Ok(())
}

fn infer_role(
    name: &str,
    _svc: &craterun_bundle::compose::compose_file::ComposeService,
) -> ServiceRole {
    let lower = name.to_lowercase();
    if lower.contains("web") || lower.contains("frontend") || lower.contains("ui") {
        ServiceRole::Frontend
    } else if lower.contains("api") || lower.contains("backend") || lower.contains("server") {
        ServiceRole::Backend
    } else if lower.contains("db") || lower.contains("postgres") || lower.contains("mysql") || lower.contains("redis") || lower.contains("mongo") {
        ServiceRole::Database
    } else {
        ServiceRole::Worker
    }
}

fn extract_env(
    svc: &craterun_bundle::compose::compose_file::ComposeService,
) -> indexmap::IndexMap<String, String> {
    let mut env = indexmap::IndexMap::new();
    if let Some(ref compose_env) = svc.environment {
        match compose_env {
            craterun_bundle::compose::compose_file::ComposeEnvironment::Map(map) => {
                for (k, v) in map {
                    env.insert(k.clone(), serde_yaml_value_to_string(v));
                }
            }
            craterun_bundle::compose::compose_file::ComposeEnvironment::List(list) => {
                for item in list {
                    if let Some((k, v)) = item.split_once('=') {
                        env.insert(k.to_string(), v.to_string());
                    }
                }
            }
        }
    }
    env
}

fn serde_yaml_value_to_string(v: &serde_yaml::Value) -> String {
    match v {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Null => String::new(),
        other => format!("{:?}", other),
    }
}
