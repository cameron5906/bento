use super::compose_file::ComposeFile;
use crate::manifest::AppManifest;

const DANGEROUS_CAPABILITIES: &[&str] = &[
    "SYS_ADMIN",
    "SYS_PTRACE",
    "SYS_RAWIO",
    "NET_ADMIN",
    "NET_RAW",
    "SYS_MODULE",
    "DAC_OVERRIDE",
    "MKNOD",
    "AUDIT_WRITE",
    "SETFCAP",
];

const DOCKER_SOCKET_PATHS: &[&str] = &[
    "/var/run/docker.sock",
    "/run/docker.sock",
    "/var/run/podman/podman.sock",
];

#[derive(Debug, Clone)]
pub struct ValidationViolation {
    pub service: Option<String>,
    pub rule: BlockedRule,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockedRule {
    PrivilegedContainer,
    HostNetworking,
    PidHost,
    IpcHost,
    DangerousCapability,
    DockerSocketMount,
    HostRootMount,
    ArbitraryBindMount,
    FixedHostPort,
    ExternalNetwork,
    ExternalVolume,
    MissingHealthCheck,
    MissingAppName,
    MissingIcon,
    MissingFrontendRoute,
}

impl std::fmt::Display for ValidationViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.service {
            Some(svc) => write!(f, "service '{}': {}", svc, self.detail),
            None => write!(f, "{}", self.detail),
        }
    }
}

pub fn validate_consumer_subset(
    compose: &ComposeFile,
    manifest: &AppManifest,
) -> Vec<ValidationViolation> {
    let mut violations = Vec::new();

    for (name, service) in &compose.services {
        if service.privileged == Some(true) {
            violations.push(ValidationViolation {
                service: Some(name.clone()),
                rule: BlockedRule::PrivilegedContainer,
                detail: "uses privileged mode".into(),
            });
        }

        if let Some(ref network_mode) = service.network_mode {
            if network_mode == "host" {
                violations.push(ValidationViolation {
                    service: Some(name.clone()),
                    rule: BlockedRule::HostNetworking,
                    detail: "uses host networking".into(),
                });
            }
        }

        if service.pid.as_deref() == Some("host") {
            violations.push(ValidationViolation {
                service: Some(name.clone()),
                rule: BlockedRule::PidHost,
                detail: "uses host PID namespace".into(),
            });
        }

        if service.ipc.as_deref() == Some("host") {
            violations.push(ValidationViolation {
                service: Some(name.clone()),
                rule: BlockedRule::IpcHost,
                detail: "uses host IPC namespace".into(),
            });
        }

        for cap in &service.cap_add {
            let cap_upper = cap.to_uppercase();
            if DANGEROUS_CAPABILITIES.contains(&cap_upper.as_str()) {
                violations.push(ValidationViolation {
                    service: Some(name.clone()),
                    rule: BlockedRule::DangerousCapability,
                    detail: format!("adds dangerous capability: {}", cap),
                });
            }
        }

        for vol in &service.volumes {
            let host_path = vol.split(':').next().unwrap_or("");

            for sock in DOCKER_SOCKET_PATHS {
                if host_path == *sock {
                    violations.push(ValidationViolation {
                        service: Some(name.clone()),
                        rule: BlockedRule::DockerSocketMount,
                        detail: format!("mounts Docker socket: {}", sock),
                    });
                }
            }

            if host_path.starts_with('/') && !is_named_volume(host_path) {
                if host_path == "/" || host_path.starts_with("/etc") || host_path.starts_with("/root") {
                    violations.push(ValidationViolation {
                        service: Some(name.clone()),
                        rule: BlockedRule::HostRootMount,
                        detail: format!("mounts sensitive host path: {}", host_path),
                    });
                } else if !DOCKER_SOCKET_PATHS.contains(&host_path) {
                    violations.push(ValidationViolation {
                        service: Some(name.clone()),
                        rule: BlockedRule::ArbitraryBindMount,
                        detail: format!("uses bind mount: {}", host_path),
                    });
                }
            }
        }

        for port in &service.ports {
            if let Some(host_port) = port.has_fixed_host_port() {
                violations.push(ValidationViolation {
                    service: Some(name.clone()),
                    rule: BlockedRule::FixedHostPort,
                    detail: format!("maps fixed host port {}", host_port),
                });
            }
        }
    }

    for (name, config) in &compose.networks {
        if let Some(cfg) = config {
            if cfg.external == Some(true) {
                violations.push(ValidationViolation {
                    service: None,
                    rule: BlockedRule::ExternalNetwork,
                    detail: format!("network '{}' is external", name),
                });
            }
        }
    }

    for (name, config) in &compose.volumes {
        if let Some(cfg) = config {
            if cfg.external == Some(true) {
                violations.push(ValidationViolation {
                    service: None,
                    rule: BlockedRule::ExternalVolume,
                    detail: format!("volume '{}' is external", name),
                });
            }
        }
    }

    if manifest.app.name.is_empty() {
        violations.push(ValidationViolation {
            service: None,
            rule: BlockedRule::MissingAppName,
            detail: "app name is not configured".into(),
        });
    }

    if manifest.app.icon.is_none() {
        violations.push(ValidationViolation {
            service: None,
            rule: BlockedRule::MissingIcon,
            detail: "app icon is not configured".into(),
        });
    }

    if manifest.routes.is_empty() {
        violations.push(ValidationViolation {
            service: None,
            rule: BlockedRule::MissingFrontendRoute,
            detail: "no frontend route configured".into(),
        });
    }

    violations
}

fn is_named_volume(path: &str) -> bool {
    !path.contains('/') && !path.contains('\\')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::compose_file::ComposeFile;
    use crate::manifest::app_manifest::AppManifest;

    fn sample_manifest() -> AppManifest {
        let yaml = r#"
app:
  id: com.example.test
  name: Test App
  version: 1.0.0
  icon: ./icon.png
compose:
  file: ./docker-compose.yml
window:
  title: Test
routes:
  /:
    service: web
    port: 3000
health:
  ready:
    service: api
    path: /health
"#;
        AppManifest::from_str(yaml).unwrap()
    }

    #[test]
    fn clean_compose_passes() {
        let compose = ComposeFile::from_str(
            r#"
services:
  web:
    image: myapp-web
    ports:
      - "3000"
  db:
    image: postgres:16
    volumes:
      - db-data:/var/lib/postgresql/data
volumes:
  db-data:
"#,
        )
        .unwrap();
        let violations = validate_consumer_subset(&compose, &sample_manifest());
        assert!(violations.is_empty(), "unexpected violations: {:?}", violations);
    }

    #[test]
    fn privileged_blocked() {
        let compose = ComposeFile::from_str(
            r#"
services:
  api:
    image: myapp
    privileged: true
"#,
        )
        .unwrap();
        let violations = validate_consumer_subset(&compose, &sample_manifest());
        assert!(violations
            .iter()
            .any(|v| v.rule == BlockedRule::PrivilegedContainer));
    }

    #[test]
    fn host_networking_blocked() {
        let compose = ComposeFile::from_str(
            r#"
services:
  api:
    image: myapp
    network_mode: host
"#,
        )
        .unwrap();
        let violations = validate_consumer_subset(&compose, &sample_manifest());
        assert!(violations
            .iter()
            .any(|v| v.rule == BlockedRule::HostNetworking));
    }

    #[test]
    fn docker_socket_blocked() {
        let compose = ComposeFile::from_str(
            r#"
services:
  worker:
    image: myapp
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
"#,
        )
        .unwrap();
        let violations = validate_consumer_subset(&compose, &sample_manifest());
        assert!(violations
            .iter()
            .any(|v| v.rule == BlockedRule::DockerSocketMount));
    }

    #[test]
    fn fixed_host_port_blocked() {
        let compose = ComposeFile::from_str(
            r#"
services:
  web:
    image: myapp
    ports:
      - "3000:3000"
"#,
        )
        .unwrap();
        let violations = validate_consumer_subset(&compose, &sample_manifest());
        assert!(violations
            .iter()
            .any(|v| v.rule == BlockedRule::FixedHostPort));
    }

    #[test]
    fn external_volume_blocked() {
        let compose = ComposeFile::from_str(
            r#"
services:
  web:
    image: myapp
volumes:
  shared:
    external: true
"#,
        )
        .unwrap();
        let violations = validate_consumer_subset(&compose, &sample_manifest());
        assert!(violations
            .iter()
            .any(|v| v.rule == BlockedRule::ExternalVolume));
    }
}
