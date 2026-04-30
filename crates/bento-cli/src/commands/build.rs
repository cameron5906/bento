use std::path::PathBuf;

use clap::Args;

use bento_bundle::bundle::BundleWriter;
use bento_bundle::compose::ComposeFile;
use bento_bundle::manifest::compiled_manifest::{CompiledManifest, ServiceEntry, ServiceVolumeMount};
use bento_bundle::manifest::AppManifest;
use bento_core::types::ServiceRole;

use crate::builder::image_pipeline;
use crate::output;

#[derive(Args)]
pub struct BuildArgs {
    /// Path to bento.yml
    #[arg(short, long, default_value = "./bento.yml")]
    pub manifest: PathBuf,

    /// Output directory for the app bundle
    #[arg(short, long, default_value = "./dist/bundle")]
    pub output: PathBuf,

    /// Target platform (auto-detected from current OS)
    #[arg(long, default_value_t = crate::platform::default_build_target())]
    pub target: String,

    /// Skip image build/export (only generate manifest)
    #[arg(long)]
    pub skip_images: bool,
}

pub async fn run(args: BuildArgs) -> anyhow::Result<()> {
    output::header("Bento Build");

    let manifest = AppManifest::from_file(&args.manifest)?;
    output::success(&format!("Loaded manifest: {}", manifest.app.name));

    let manifest_dir = args
        .manifest
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf();

    let compose_path = manifest_dir.join(&manifest.compose.file);
    let compose = ComposeFile::from_file(&compose_path)?;
    output::success(&format!(
        "Loaded compose file: {} services",
        compose.services.len()
    ));

    let writer = BundleWriter::new(&args.output);
    writer.ensure_dirs()?;

    let compose_dir = compose_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    let project_name = manifest
        .compose
        .project_name
        .as_deref()
        .unwrap_or(manifest.app.id.as_str());

    // Build and export images
    let mut build_results = Vec::new();
    if !args.skip_images {
        for (name, svc) in &compose.services {
            match image_pipeline::build_and_export(
                name,
                svc,
                project_name,
                compose_dir,
                &writer.images_dir(),
            )
            .await
            {
                Ok(result) => {
                    output::success(&format!("Exported: {} ({})", result.image_tag, result.digest));
                    build_results.push(result);
                }
                Err(e) => {
                    output::failure(&format!("Failed to build '{}': {}", name, e));
                    return Err(e);
                }
            }
        }
    } else {
        output::info("--skip-images: skipping image build/export.");
    }

    // Build service entries for the compiled manifest, using digest from build
    let services: Vec<ServiceEntry> = compose
        .services
        .iter()
        .map(|(name, svc)| {
            let port = svc
                .ports
                .first()
                .and_then(|p| p.container_port())
                .unwrap_or(0);

            let build_result = build_results
                .iter()
                .find(|r| r.service_name == *name);

            ServiceEntry {
                name: name.clone(),
                image_tag: build_result.map(|r| r.image_tag.clone()),
                image_archive: build_result
                    .map(|r| r.archive_name.clone())
                    .unwrap_or_else(|| format!("images/{}-linux-amd64.oci.tar.zst", name)),
                image_digest: build_result.map(|r| r.digest.clone()),
                container_port: port,
                role: infer_role(name),
                env: extract_env(svc),
                depends_on: compose.depends_on_list(name),
                restart_policy: Default::default(),
                volume_mounts: extract_volume_mounts(svc),
            }
        })
        .collect();

    let compiled = CompiledManifest::from_app_manifest(&manifest, services, &args.target);

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

    // Copy icon if it exists
    if let Some(ref icon) = manifest.app.icon {
        let icon_path = manifest_dir.join(icon);
        if icon_path.exists() {
            writer.copy_asset(&icon_path, "icon.png")?;
            output::success("Copied icon");
        }
    }

    if !build_results.is_empty() {
        image_pipeline::print_size_report(&build_results);
    }

    output::success(&format!("Bundle output: {}", args.output.display()));
    Ok(())
}

fn infer_role(name: &str) -> ServiceRole {
    let lower = name.to_lowercase();
    if lower.contains("web") || lower.contains("frontend") || lower.contains("ui") {
        ServiceRole::Frontend
    } else if lower.contains("api") || lower.contains("backend") || lower.contains("server") {
        ServiceRole::Backend
    } else if lower.contains("db")
        || lower.contains("postgres")
        || lower.contains("mysql")
        || lower.contains("redis")
        || lower.contains("mongo")
    {
        ServiceRole::Database
    } else {
        ServiceRole::Worker
    }
}

fn extract_env(
    svc: &bento_bundle::compose::compose_file::ComposeService,
) -> indexmap::IndexMap<String, String> {
    let mut env = indexmap::IndexMap::new();
    if let Some(ref compose_env) = svc.environment {
        match compose_env {
            bento_bundle::compose::compose_file::ComposeEnvironment::Map(map) => {
                for (k, v) in map {
                    env.insert(k.clone(), serde_yaml_value_to_string(v));
                }
            }
            bento_bundle::compose::compose_file::ComposeEnvironment::List(list) => {
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

/// Parse named volume mounts from compose service (e.g. "db-data:/var/lib/postgresql/data")
fn extract_volume_mounts(
    svc: &bento_bundle::compose::compose_file::ComposeService,
) -> Vec<ServiceVolumeMount> {
    svc.volumes
        .iter()
        .filter_map(|v| {
            let parts: Vec<&str> = v.splitn(2, ':').collect();
            if parts.len() == 2 && !parts[0].starts_with('/') && !parts[0].starts_with('.') {
                // Named volume (not a bind mount path)
                Some(ServiceVolumeMount {
                    name: parts[0].to_string(),
                    mount_path: parts[1].to_string(),
                })
            } else {
                None
            }
        })
        .collect()
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
