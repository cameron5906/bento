//! Builds Docker images, exports them as OCI tar archives, and compresses
//! with zstd for inclusion in the .appcrate bundle.
//!
//! Pipeline per service:
//!   1. `docker build -t <tag> <context>` (only for services with a `build:` key)
//!   2. `docker inspect --format {{.Id}} <tag>` to pin the digest
//!   3. `docker save <tag>` piped to a .tar file
//!   4. zstd compression of the .tar → .tar.zst
//!
//! Services with only `image:` (like postgres:16) are pulled and exported as-is.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use bento_bundle::compose::compose_file::{ComposeBuild, ComposeService};

use crate::output;

pub struct ImageBuildResult {
    pub service_name: String,
    pub image_tag: String,
    pub digest: String,
    /// Relative path inside the bundle (e.g. "images/web-linux-amd64.oci.tar")
    pub archive_name: String,
    pub archive_path: PathBuf,
    pub archive_size: u64,
}

/// Build (or pull) an image and export it as a compressed OCI archive.
pub async fn build_and_export(
    service_name: &str,
    service: &ComposeService,
    project_name: &str,
    compose_dir: &Path,
    images_dir: &Path,
) -> anyhow::Result<ImageBuildResult> {
    let image_tag = format!("{}-{}:latest", project_name, service_name);
    let archive_name = format!("{}-linux-amd64.oci.tar.zst", service_name);
    let archive_path = images_dir.join(&archive_name);

    // Step 1: Build or pull
    if let Some(ref build_config) = service.build {
        let context = match build_config {
            ComposeBuild::Simple(ctx) => compose_dir.join(ctx),
            ComposeBuild::Detailed(cfg) => compose_dir.join(&cfg.context),
        };

        let dockerfile_args: Vec<String> = match build_config {
            ComposeBuild::Detailed(cfg) if cfg.dockerfile.is_some() => {
                vec![
                    "-f".to_string(),
                    context
                        .join(cfg.dockerfile.as_ref().unwrap())
                        .to_string_lossy()
                        .to_string(),
                ]
            }
            _ => vec![],
        };

        output::info(&format!("Building image: {} from {}", image_tag, context.display()));

        let mut args = vec!["build".to_string(), "-t".to_string(), image_tag.clone()];
        // Build for linux/amd64 since containers run inside WSL
        args.extend(["--platform".to_string(), "linux/amd64".to_string()]);
        args.extend(dockerfile_args);
        args.push(context.to_string_lossy().to_string());

        let status = tokio::process::Command::new("docker")
            .args(&args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;

        if !status.success() {
            anyhow::bail!("docker build failed for service '{}'", service_name);
        }
    } else if let Some(ref image) = service.image {
        // Pull the image if it has a remote image reference
        output::info(&format!("Pulling image: {}", image));

        let status = tokio::process::Command::new("docker")
            .args(["pull", "--platform", "linux/amd64", image])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;

        if !status.success() {
            anyhow::bail!("docker pull failed for image '{}'", image);
        }

        // Tag it with our project name for consistent export
        let tag_status = tokio::process::Command::new("docker")
            .args(["tag", image, &image_tag])
            .status()
            .await?;

        if !tag_status.success() {
            anyhow::bail!("docker tag failed for '{}'", image);
        }
    } else {
        anyhow::bail!(
            "service '{}' has neither build: nor image: — cannot export",
            service_name
        );
    }

    // Step 2: Get digest
    let digest_output = tokio::process::Command::new("docker")
        .args(["inspect", "--format", "{{.Id}}", &image_tag])
        .output()
        .await?;

    let digest = String::from_utf8_lossy(&digest_output.stdout)
        .trim()
        .to_string();

    // Step 3 + 4: docker save | zstd > archive
    // We save to a temp .tar then compress, since piping docker save
    // directly to zstd can be unreliable on Windows
    let tar_path = images_dir.join(format!("{}-linux-amd64.oci.tar", service_name));

    output::info(&format!("Exporting image: {} -> {}", image_tag, archive_name));

    let save_status = tokio::process::Command::new("docker")
        .args(["save", "-o", &tar_path.to_string_lossy(), &image_tag])
        .status()
        .await?;

    if !save_status.success() {
        anyhow::bail!("docker save failed for '{}'", image_tag);
    }

    // Compress with zstd if available, otherwise keep the .tar
    let final_path = compress_with_zstd(&tar_path, &archive_path).await;

    // Use the actual filename that was produced (may be .tar instead of .tar.zst)
    let actual_archive_name = final_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let archive_size = std::fs::metadata(&final_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(ImageBuildResult {
        service_name: service_name.to_string(),
        image_tag,
        digest,
        archive_name: format!("images/{}", actual_archive_name),
        archive_path: final_path,
        archive_size,
    })
}

/// Try to compress a .tar to .tar.zst using the zstd CLI.
/// Falls back to keeping the uncompressed .tar if zstd isn't available.
async fn compress_with_zstd(tar_path: &Path, zst_path: &Path) -> PathBuf {
    let result = tokio::process::Command::new("zstd")
        .args([
            "-q",             // quiet
            "--rm",           // remove source after compression
            "-o",
            &zst_path.to_string_lossy(),
            &tar_path.to_string_lossy(),
        ])
        .status()
        .await;

    match result {
        Ok(s) if s.success() => zst_path.to_path_buf(),
        _ => {
            // zstd not available — rename .tar to the expected .tar.zst path
            // so the manifest references still work. The supervisor handles
            // both compressed and uncompressed archives.
            let fallback = tar_path.with_extension("tar");
            if fallback.exists() {
                return fallback;
            }
            tar_path.to_path_buf()
        }
    }
}

/// Print a bundle size report matching the spec format.
pub fn print_size_report(results: &[ImageBuildResult]) {
    output::header("Bundle Size Report");
    let mut total: u64 = 0;
    for r in results {
        let mb = r.archive_size as f64 / 1_048_576.0;
        output::info(&format!("{}: {:.1} MB", r.service_name, mb));
        total += r.archive_size;
    }
    let total_mb = total as f64 / 1_048_576.0;
    output::info(&format!("total: {:.1} MB", total_mb));
}
