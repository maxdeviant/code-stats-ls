use std::time::Duration;

use anyhow::Result;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Serialize, Deserialize)]
pub struct Pulse {
    pub coded_at: String,
    pub xps: Vec<PulseXp>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PulseXp {
    pub language: String,
    pub xp: u32,
}

/// Sends pulses to the Code::Stats API.
pub struct PulseSender {
    http_client: reqwest::Client,
    api_url: Url,
    api_token: String,
    timeout: Duration,
}

impl PulseSender {
    pub fn new(config: Config, timeout: Duration) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            api_url: config.api_url,
            api_token: config.api_token,
            timeout,
        }
    }

    pub async fn send(&self, pulse: &Pulse, user_agent: &str) -> Result<()> {
        let mut pulse_url = self.api_url.clone();
        pulse_url.set_path("/api/my/pulses");

        self.http_client
            .post(pulse_url)
            .timeout(self.timeout)
            .header("User-Agent", user_agent)
            .header("X-API-Token", &self.api_token)
            .json(&pulse)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}
