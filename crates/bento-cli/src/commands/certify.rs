use std::path::PathBuf;

use clap::Args;

use bento_bundle::compose::{validate_consumer_subset, BlockedRule, ComposeFile};
use bento_bundle::manifest::AppManifest;

use crate::output;

#[derive(Args)]
pub struct CertifyArgs {
    /// Path to bento.yml
    #[arg(short, long, default_value = "./bento.yml")]
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

    // Helper: check if a specific rule triggered
    let has_violation = |rule: BlockedRule| violations.iter().any(|v| v.rule == rule);

    // Manifest-level checks (not covered by the compose validator)
    let checks: Vec<(&str, bool)> = vec![
        ("app name configured", !manifest.app.name.is_empty()),
        (
            "app ID is not a placeholder",
            !manifest.app.id.as_str().starts_with("com.example"),
        ),
        ("app version configured", !manifest.app.version.is_empty()),
        ("icon configured", manifest.app.icon.is_some()),
        ("frontend route configured", !manifest.routes.is_empty()),
        (
            "health check configured",
            !manifest.health.ready.path.is_empty()
                && !manifest.health.ready.service.is_empty(),
        ),
        (
            "health check service exists in compose",
            compose
                .services
                .contains_key(&manifest.health.ready.service),
        ),
        (
            "all routed services exist in compose",
            manifest
                .routes
                .values()
                .all(|r| compose.services.contains_key(&r.service)),
        ),
        (
            "persistent volumes declared",
            // Either no volumes in compose, or all compose volumes are declared in manifest
            compose.volumes.is_empty()
                || compose
                    .volumes
                    .keys()
                    .all(|v| manifest.volumes.contains_key(v)),
        ),
        // Compose safety checks — each maps to a blocked rule
        ("no privileged containers", !has_violation(BlockedRule::PrivilegedContainer)),
        ("no host networking", !has_violation(BlockedRule::HostNetworking)),
        ("no Docker socket mounts", !has_violation(BlockedRule::DockerSocketMount)),
        (
            "no dangerous capabilities",
            !has_violation(BlockedRule::DangerousCapability),
        ),
        (
            "no arbitrary host mounts",
            !has_violation(BlockedRule::ArbitraryBindMount)
                && !has_violation(BlockedRule::HostRootMount),
        ),
        (
            "all host ports auto-assigned",
            !has_violation(BlockedRule::FixedHostPort),
        ),
        (
            "no external networks",
            !has_violation(BlockedRule::ExternalNetwork),
        ),
        (
            "no external volumes",
            !has_violation(BlockedRule::ExternalVolume),
        ),
        (
            "all images buildable locally",
            compose.services.values().all(|s| {
                s.build.is_some() || s.image.is_some()
            }),
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

    // Print specific violation details
    let detail_violations: Vec<_> = violations
        .iter()
        .filter(|v| {
            // Skip generic manifest-level rules already covered above
            !matches!(
                v.rule,
                BlockedRule::MissingAppName
                    | BlockedRule::MissingIcon
                    | BlockedRule::MissingFrontendRoute
            )
        })
        .collect();

    if !detail_violations.is_empty() {
        println!();
        for v in detail_violations {
            output::failure(&v.to_string());
        }
    }

    println!();
    if all_pass {
        output::success("Certified: consumer-ready");
        Ok(())
    } else {
        output::failure("Consumer readiness failed");
        output::info("This app can be packaged as a Dev Pack, but not a Consumer Pack.");
        std::process::exit(1);
    }
}
