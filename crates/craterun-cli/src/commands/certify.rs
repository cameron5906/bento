use std::path::PathBuf;

use clap::Args;

use craterun_bundle::compose::{validate_consumer_subset, ComposeFile};
use craterun_bundle::manifest::AppManifest;

use crate::output;

#[derive(Args)]
pub struct CertifyArgs {
    /// Path to craterun.yml
    #[arg(short, long, default_value = "./craterun.yml")]
    pub manifest: PathBuf,
}

pub async fn run(args: CertifyArgs) -> anyhow::Result<()> {
    output::header("Consumer Readiness Report");

    let manifest = AppManifest::from_file(&args.manifest)?;

    let compose_path = args
        .manifest
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(&manifest.compose.file);

    let compose = ComposeFile::from_file(&compose_path)?;

    let violations = validate_consumer_subset(&compose, &manifest);

    let checks = vec![
        ("app name configured", manifest.app.name.len() > 0),
        ("icon configured", manifest.app.icon.is_some()),
        ("frontend route configured", !manifest.routes.is_empty()),
        (
            "health check configured",
            !manifest.health.ready.path.is_empty(),
        ),
        (
            "no privileged containers",
            !violations
                .iter()
                .any(|v| v.rule == craterun_bundle::compose::BlockedRule::PrivilegedContainer),
        ),
        (
            "no host networking",
            !violations
                .iter()
                .any(|v| v.rule == craterun_bundle::compose::BlockedRule::HostNetworking),
        ),
        (
            "no Docker socket mounts",
            !violations
                .iter()
                .any(|v| v.rule == craterun_bundle::compose::BlockedRule::DockerSocketMount),
        ),
        (
            "no arbitrary host mounts",
            !violations
                .iter()
                .any(|v| v.rule == craterun_bundle::compose::BlockedRule::ArbitraryBindMount
                    || v.rule == craterun_bundle::compose::BlockedRule::HostRootMount),
        ),
        (
            "all host ports auto-assigned",
            !violations
                .iter()
                .any(|v| v.rule == craterun_bundle::compose::BlockedRule::FixedHostPort),
        ),
    ];

    let mut all_pass = true;
    for (label, passed) in &checks {
        if *passed {
            output::success(label);
        } else {
            output::failure(label);
            all_pass = false;
        }
    }

    if !violations.is_empty() {
        println!();
        for v in &violations {
            output::failure(&v.to_string());
        }
    }

    println!();
    if all_pass && violations.is_empty() {
        output::success("Certified: consumer-ready");
        Ok(())
    } else {
        output::failure("Consumer readiness failed");
        output::info("This app can be packaged as a Dev Pack, but not a Consumer Pack.");
        std::process::exit(1);
    }
}
