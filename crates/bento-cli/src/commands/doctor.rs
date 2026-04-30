use crate::output;

pub async fn run() -> anyhow::Result<()> {
    output::header("Bento Doctor");

    check_docker().await;

    #[cfg(windows)]
    check_wsl().await;

    #[cfg(target_os = "linux")]
    check_podman().await;

    check_bento_yml();

    println!();
    Ok(())
}

async fn check_docker() {
    let result = bento_runtime::detect::detect_docker().await;
    if result.available {
        output::success(&format!(
            "Docker: available ({})",
            result.version.unwrap_or_else(|| "unknown version".into())
        ));
    } else {
        output::failure(&format!(
            "Docker: {}",
            result.blocker.unwrap_or_else(|| "not found".into())
        ));
    }
}

#[cfg(windows)]
async fn check_wsl() {
    let result = bento_runtime::detect::detect_wsl().await;
    if result.available {
        output::success("WSL2: available");
    } else {
        let msg = result.blocker.unwrap_or_else(|| "not available".into());
        output::info(&format!("WSL2: {} (only required for consumer builds)", msg));
    }
}

#[cfg(target_os = "linux")]
async fn check_podman() {
    let result = bento_runtime::detect::detect_podman().await;
    if result.available {
        output::success(&format!(
            "Podman: available ({})",
            result.version.unwrap_or_else(|| "unknown".into())
        ));
    } else {
        output::info("Podman: not found (Docker is sufficient)");
    }
}

fn check_bento_yml() {
    let path = std::path::Path::new("bento.yml");
    if path.exists() {
        match bento_bundle::manifest::AppManifest::from_file(path) {
            Ok(m) => output::success(&format!("bento.yml: valid (app: {})", m.app.name)),
            Err(e) => output::failure(&format!("bento.yml: {}", e)),
        }
    } else {
        output::info("bento.yml: not found (run 'bento init' to create one)");
    }
}
