use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AppId(String);

impl AppId {
    pub fn new(id: &str) -> Result<Self, InvalidAppId> {
        Self::validate(id)?;
        Ok(Self(id.to_string()))
    }

    fn validate(id: &str) -> Result<(), InvalidAppId> {
        if id.is_empty() {
            return Err(InvalidAppId("app ID cannot be empty".into()));
        }

        let parts: Vec<&str> = id.split('.').collect();
        if parts.len() < 2 {
            return Err(InvalidAppId(
                "app ID must be reverse-domain format (e.g. com.example.myapp)".into(),
            ));
        }

        for part in &parts {
            if part.is_empty() {
                return Err(InvalidAppId("app ID segments cannot be empty".into()));
            }
            if !part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return Err(InvalidAppId(format!(
                    "app ID segment '{}' contains invalid characters",
                    part
                )));
            }
        }

        Ok(())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn distro_name(&self) -> String {
        format!("bento-{}", self.0.replace('.', "-"))
    }
}

impl fmt::Display for AppId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for AppId {
    type Error = InvalidAppId;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        AppId::new(&s)
    }
}

impl From<AppId> for String {
    fn from(id: AppId) -> Self {
        id.0
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid app ID: {0}")]
pub struct InvalidAppId(String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_app_ids() {
        assert!(AppId::new("com.example.myapp").is_ok());
        assert!(AppId::new("io.github.user.project").is_ok());
        assert!(AppId::new("com.my-company.app_name").is_ok());
    }

    #[test]
    fn invalid_app_ids() {
        assert!(AppId::new("").is_err());
        assert!(AppId::new("singleword").is_err());
        assert!(AppId::new("com..empty").is_err());
        assert!(AppId::new("com.ex ample.bad").is_err());
    }

    #[test]
    fn distro_name() {
        let id = AppId::new("com.example.photobooth").unwrap();
        assert_eq!(id.distro_name(), "bento-com-example-photobooth");
    }
}
