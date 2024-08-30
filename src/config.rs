use std::io::ErrorKind;
use std::{env, fs};

use anyhow::{anyhow, Context, Result};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use toml;

#[derive(Debug)]
pub struct Config {
    pub api_url: Url,
    pub api_token: String,
}

impl Config {
    pub fn read() -> Result<Self> {
        let config_toml = ConfigToml::try_read()?;

        let api_url = env::var("CODE_STATS_API_URL")
            .ok()
            .or_else(|| {
                config_toml
                    .as_ref()
                    .and_then(|config| config.api_url.clone())
            })
            .unwrap_or("https://codestats.net".to_string());

        let api_url =
            Url::parse(&api_url).with_context(|| anyhow!("invalid API URL: {api_url}"))?;
        let api_token = env::var("CODE_STATS_API_TOKEN")
            .ok()
            .or_else(|| {
                config_toml
                    .as_ref()
                    .and_then(|config| config.api_token.clone())
            })
            .context("CODE_STATS_API_TOKEN must be set")?;

        Ok(Self { api_url, api_token })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ConfigToml {
    pub api_url: Option<String>,
    pub api_token: Option<String>,
}

impl ConfigToml {
    pub fn try_read() -> Result<Option<Self>> {
        let home_dir = dirs::home_dir().context("no home directory found")?;
        let config_dir = home_dir.join(".config/code-stats");
        let config_toml_path = config_dir.join("config.toml");

        fs::create_dir_all(&config_dir).context("failed to create config directory")?;

        let config_toml = match fs::read_to_string(&config_toml_path) {
            Ok(toml) => toml::from_str(&toml)?,
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    return Ok(None);
                }

                return Err(err).context("failed to read config.toml");
            }
        };

        Ok(Some(config_toml))
    }
}
