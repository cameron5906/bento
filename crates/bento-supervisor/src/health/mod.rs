use std::time::{Duration, Instant};

use bento_bundle::manifest::compiled_manifest::CompiledManifest;
use bento_core::BentoError;
use bento_runtime::types::RuntimePlan;

pub struct HealthChecker {
    service_name: String,
    health_url: String,
    timeout: Duration,
    interval: Duration,
}

impl HealthChecker {
    pub fn new(manifest: &CompiledManifest, plan: &RuntimePlan) -> Self {
        let health = &manifest.health.ready;

        let host_port = plan
            .services
            .iter()
            .find(|s| s.name == health.service)
            .map(|s| s.host_port)
            .unwrap_or(8080);

        let health_url = format!("http://127.0.0.1:{}{}", host_port, health.path);

        Self {
            service_name: health.service.clone(),
            health_url,
            timeout: Duration::from_secs(health.timeout_seconds as u64),
            interval: Duration::from_secs(2),
        }
    }

    pub async fn wait_until_ready(&self) -> Result<(), BentoError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| BentoError::RuntimeError(format!("http client error: {}", e)))?;

        let start = Instant::now();

        loop {
            if start.elapsed() > self.timeout {
                return Err(BentoError::HealthCheckTimeout {
                    service: self.service_name.clone(),
                    elapsed_seconds: start.elapsed().as_secs() as u32,
                });
            }

            match client.get(&self.health_url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    tracing::info!(
                        "health check passed for '{}' after {:.1}s",
                        self.service_name,
                        start.elapsed().as_secs_f64()
                    );
                    return Ok(());
                }
                Ok(resp) => {
                    tracing::debug!(
                        "health check for '{}': status {}",
                        self.service_name,
                        resp.status()
                    );
                }
                Err(e) => {
                    tracing::debug!(
                        "health check for '{}': {}",
                        self.service_name,
                        e
                    );
                }
            }

            tokio::time::sleep(self.interval).await;
        }
    }
}
