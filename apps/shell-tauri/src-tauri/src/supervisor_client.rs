use reqwest::Client;

use craterun_core::types::StatusResponse;

pub struct SupervisorClient {
    client: Client,
    base_url: String,
    token: String,
}

impl SupervisorClient {
    pub fn new(port: u16, token: String) -> Self {
        Self {
            client: Client::new(),
            base_url: format!("http://127.0.0.1:{}", port),
            token,
        }
    }

    pub async fn get_status(&self) -> Result<StatusResponse, String> {
        self.client
            .get(format!("{}/status", self.base_url))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await
            .map_err(|e| format!("connection failed: {}", e))?
            .json::<StatusResponse>()
            .await
            .map_err(|e| format!("invalid response: {}", e))
    }

    pub async fn post_command(&self, command: &str) -> Result<(), String> {
        let url = format!("{}/{}", self.base_url, command);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await
            .map_err(|e| format!("request failed: {}", e))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!("command failed: HTTP {}", resp.status()))
        }
    }
}
