use crate::output;

pub async fn run() -> anyhow::Result<()> {
    output::header("CrateRun Doctor");

    check_docker().await;
    check_wsl().await;
    check_craterun_yml();

    println!();
    Ok(())
}

async fn check_docker() {
    let result = craterun_runtime::detect::detect_docker().await;
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

async fn check_wsl() {
    let result = craterun_runtime::detect::detect_wsl().await;
    if result.available {
        output::success("WSL2: available");
    } else {
        let msg = result.blocker.unwrap_or_else(|| "not available".into());
        output::info(&format!("WSL2: {} (only required for consumer builds)", msg));
    }
}

fn check_craterun_yml() {
    let path = std::path::Path::new("craterun.yml");
    if path.exists() {
        match craterun_bundle::manifest::AppManifest::from_file(path) {
            Ok(m) => output::success(&format!("craterun.yml: valid (app: {})", m.app.name)),
            Err(e) => output::failure(&format!("craterun.yml: {}", e)),
        }
    } else {
        output::info("craterun.yml: not found (run 'craterun init' to create one)");
    }
}
