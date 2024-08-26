use chrono::Utc;
use dotenv::dotenv;
use serde_json::json;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct CodeStatsLanguageServer {
    client: Client,
    xp_count: Mutex<u32>,
    api_token: String,
}

#[tower_lsp::async_trait]
impl LanguageServer for CodeStatsLanguageServer {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult::default())
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
        let content_changes = params.content_changes;
        let xp_gained = content_changes.len() as u32;

        let mut xp_count = self.xp_count.lock().await;
        *xp_count += xp_gained;

        if *xp_count >= 100 {
            self.send_pulse(*xp_count).await;
            *xp_count = 0;
        }
    }
}

impl CodeStatsLanguageServer {
    async fn send_pulse(&self, gained_xp: u32) {
        let client = reqwest::Client::new();
        let url = "https://codestats.net/api/my/pulses";

        let payload = json!({
            "coded_at": Utc::now().to_rfc3339(),
            "xps": [
                {
                    "language": "Rust",
                    "xp": gained_xp
                }
            ]
        });

        match client
            .post(url)
            .header("X-API-Token", &self.api_token)
            .json(&payload)
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

#[tokio::main]
async fn main() {
    dotenv().ok();

    let api_token =
        std::env::var("CODE_STATS_API_TOKEN").expect("CODE_STATS_API_TOKEN must be set");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| CodeStatsLanguageServer {
        client,
        xp_count: Mutex::new(0),
        api_token,
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}
