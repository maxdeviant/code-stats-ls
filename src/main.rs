use std::collections::HashMap;

use chrono::Local;
use dotenv::dotenv;
use serde::Serialize;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug, Serialize)]
struct Pulse {
    coded_at: String,
    xps: Vec<PulseXp>,
}

#[derive(Debug, Serialize)]
struct PulseXp {
    pub language: String,
    pub xp: u32,
}

struct CodeStatsLanguageServer {
    client: Client,
    http_client: reqwest::Client,
    api_token: String,
    xp_gained_by_language: Mutex<HashMap<String, u32>>,
}

impl CodeStatsLanguageServer {
    fn language_for_document_uri(&self, uri: &Url) -> Option<String> {
        let filename = uri.path().split('/').last().unwrap_or("");
        let extension = filename.split('.').last().unwrap_or("");

        match extension {
            "gleam" => Some("Gleam".to_string()),
            "html" => Some("HTML".to_string()),
            "js" => Some("JavaScript".to_string()),
            "json" => Some("JSON".to_string()),
            "jsx" => Some("JavaScript (React)".to_string()),
            "md" | "markdown" => Some("Markdown".to_string()),
            "rs" => Some("Rust".to_string()),
            "toml" => Some("TOML".to_string()),
            "ts" => Some("TypeScript".to_string()),
            "tsx" => Some("TypeScript (React)".to_string()),
            "yaml" | "yml" => Some("YAML".to_string()),
            _ => None,
        }
    }

    async fn send_pulse(&self, xp_gained: Vec<(String, u32)>) {
        let url = "https://codestats.net/api/my/pulses";

        let pulse = Pulse {
            coded_at: Local::now().to_rfc3339(),
            xps: xp_gained
                .into_iter()
                .map(|(language, xp)| PulseXp { language, xp })
                .collect(),
        };

        match self
            .http_client
            .post(url)
            .header("X-API-Token", &self.api_token)
            .json(&pulse)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    self.client
                        .log_message(MessageType::INFO, "XP pulse sent successfully")
                        .await;
                } else {
                    self.client
                        .log_message(MessageType::ERROR, "Failed to send XP pulse")
                        .await;
                }
            }
            Err(err) => {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!("Error sending XP pulse: {}", err),
                    )
                    .await;
            }
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for CodeStatsLanguageServer {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: env!("CARGO_PKG_NAME").to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Code::Stats language server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let Some(language) = self.language_for_document_uri(&params.text_document.uri) else {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("No language for file: {}", params.text_document.uri.path()),
                )
                .await;

            return;
        };

        let content_changes = params.content_changes;
        let xp_gained = content_changes.len() as u32;

        let mut xp_gained_by_language = self.xp_gained_by_language.lock().await;
        let send_pulse = {
            let total_xp_gained = xp_gained_by_language.entry(language).or_insert(0);
            *total_xp_gained += xp_gained;

            *total_xp_gained >= 100
        };

        if send_pulse {
            let pulse = xp_gained_by_language
                .iter()
                .map(|(language, xp)| (language.clone(), *xp))
                .collect();

            self.send_pulse(pulse).await;
            xp_gained_by_language.clear();
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let api_token =
        std::env::var("CODE_STATS_API_TOKEN").expect("CODE_STATS_API_TOKEN must be set");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| CodeStatsLanguageServer {
        client,
        http_client: reqwest::Client::new(),
        xp_gained_by_language: Mutex::new(HashMap::new()),
        api_token,
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}
