mod cache;
mod config;

use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::Local;
use clap::Parser;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex, RwLock};
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::cache::PulseCache;
use crate::config::Config;

#[derive(Debug, Serialize, Deserialize)]
struct Pulse {
    coded_at: String,
    xps: Vec<PulseXp>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PulseXp {
    pub language: String,
    pub xp: u32,
}

struct CodeStatsLanguageServer {
    client: Client,
    http_client: reqwest::Client,
    config: Config,
    client_info: Arc<RwLock<Option<ClientInfo>>>,
    xp_gained_by_language: Arc<Mutex<HashMap<String, u32>>>,
    pulse_tx: mpsc::Sender<()>,
    pulse_cache: Arc<PulseCache>,
}

impl CodeStatsLanguageServer {
    pub fn new(
        client: Client,
        config: Config,
        pulse_tx: mpsc::Sender<()>,
        pulse_cache: PulseCache,
    ) -> Self {
        Self {
            client,
            http_client: reqwest::Client::new(),
            config,
            client_info: Arc::new(RwLock::new(None)),
            xp_gained_by_language: Arc::new(Mutex::new(HashMap::new())),
            pulse_tx,
            pulse_cache: Arc::new(pulse_cache),
        }
    }

    const fn name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    const fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    async fn user_agent(&self) -> String {
        let mut user_agent = format!(
            "{name}/{version}",
            name = self.name(),
            version = self.version(),
        );

        if let Some(client_info) = self.client_info.read().await.as_ref() {
            user_agent.push(' ');
            user_agent.push('(');
            user_agent.push_str(&client_info.name);

            if let Some(version) = client_info.version.as_ref() {
                user_agent.push(' ');
                user_agent.push_str(&version);
            }

            user_agent.push(')');
        }

        user_agent
    }

    fn language_for_document_uri(&self, uri: &Url) -> Option<String> {
        let filename = uri.path().split('/').last().unwrap_or("");
        let extension = filename.split('.').last().unwrap_or("");

        match extension {
            "asciidoc" | "adoc" => Some("AsciiDoc".to_string()),
            "asm" => Some("Assembly".to_string()),
            "c" | "h" => Some("C".to_string()),
            "clj" => Some("Clojure".to_string()),
            "coq" => Some("Coq".to_string()),
            "cpp" => Some("C++".to_string()),
            "cr" => Some("Crystal".to_string()),
            "cs" => Some("C#".to_string()),
            "css" => Some("CSS".to_string()),
            "csv" => Some("CSV".to_string()),
            "d" => Some("D".to_string()),
            "dart" => Some("Dart".to_string()),
            "diff" | "patch" => Some("Diff".to_string()),
            "el" => Some("Emacs Lisp".to_string()),
            "elm" => Some("Elm".to_string()),
            "erl" => Some("Erlang".to_string()),
            "ex" => Some("Elixir".to_string()),
            "fish" => Some("Fish".to_string()),
            "fs" | "fsi" | "fsx" => Some("F#".to_string()),
            "gd" => Some("GDScript".to_string()),
            "gleam" => Some("Gleam".to_string()),
            "go" => Some("Go".to_string()),
            "graphql" | "gql" => Some("GraphQL".to_string()),
            "hbs" => Some("Handlebars".to_string()),
            "heex" => Some("HTML (EEx)".to_string()),
            "hs" => Some("Haskell".to_string()),
            "html" | "htm" => Some("HTML".to_string()),
            "hx" => Some("Haxe".to_string()),
            "hy" => Some("Hy".to_string()),
            "idr" => Some("Idris".to_string()),
            "java" => Some("Java".to_string()),
            "jl" => Some("Julia".to_string()),
            "js" | "mjs" | "cjs" => Some("JavaScript".to_string()),
            "json" => Some("JSON".to_string()),
            "jsx" => Some("JavaScript (React)".to_string()),
            "kt" | "ktm" | "kts" => Some("Kotlin".to_string()),
            "less" => Some("Less".to_string()),
            "lfe" => Some("LFE".to_string()),
            "lua" => Some("Lua".to_string()),
            "md" | "markdown" => Some("Markdown".to_string()),
            "ml" => Some("OCaml".to_string()),
            "nim" => Some("Nim".to_string()),
            "nix" => Some("Nix".to_string()),
            "php" => Some("PHP".to_string()),
            "ps1" => Some("PowerShell".to_string()),
            "purs" => Some("PureScript".to_string()),
            "py" => Some("Python".to_string()),
            "rb" => Some("Ruby".to_string()),
            "rkt" => Some("Racket".to_string()),
            "roc" => Some("Roc".to_string()),
            "rs" => Some("Rust".to_string()),
            "rst" => Some("reStructuredText".to_string()),
            "scala" => Some("Scala".to_string()),
            "scm" => Some("Scheme".to_string()),
            "scss" => Some("SCSS".to_string()),
            "sh" => Some("Shell".to_string()),
            "sql" => Some("SQL".to_string()),
            "svg" => Some("SVG".to_string()),
            "swift" => Some("Swift".to_string()),
            "tex" => Some("LaTeX".to_string()),
            "toml" => Some("TOML".to_string()),
            "ts" | "mts" | "cts" => Some("TypeScript".to_string()),
            "tsx" => Some("TypeScript (React)".to_string()),
            "twig" => Some("Twig".to_string()),
            "txt" => Some("Plaintext".to_string()),
            "vala" => Some("Vala".to_string()),
            "vb" => Some("Visual Basic".to_string()),
            "vue" => Some("Vue".to_string()),
            "wit" => Some("WIT".to_string()),
            "xml" => Some("XML".to_string()),
            "yaml" | "yml" => Some("YAML".to_string()),
            "zig" => Some("Zig".to_string()),
            _ => None,
        }
    }

    async fn send_cached_pulses(&self) -> Result<()> {
        let pulses = self.pulse_cache.list()?;

        let mut sent_count = 0;

        for pulse in pulses {
            match self.send_pulse_internal(&pulse).await {
                Ok(()) => {
                    self.pulse_cache.remove(&pulse)?;
                    sent_count += 1;
                }
                Err(err) => {
                    self.client
                        .log_message(
                            MessageType::ERROR,
                            format!(
                                "Error sending cached XP pulse from {}: {err}",
                                pulse.coded_at
                            ),
                        )
                        .await;
                }
            }

            tokio::time::sleep(Duration::from_millis(250)).await;
        }

        if sent_count > 0 {
            self.client
                .log_message(
                    MessageType::INFO,
                    format!(
                        "Sent {sent_count} cached XP pulse{}",
                        if sent_count == 1 { "" } else { "s" },
                    ),
                )
                .await;
        }

        Ok(())
    }

    async fn send_pulse(&self) {
        let mut xp_gained_by_language = self.xp_gained_by_language.lock().await;

        // If we have no XP to gain, no need to send a pulse.
        if xp_gained_by_language.is_empty() {
            return;
        }

        let pulse = Pulse {
            coded_at: Local::now().to_rfc3339(),
            xps: xp_gained_by_language
                .iter()
                .map(|(language, xp)| PulseXp {
                    language: language.clone(),
                    xp: *xp,
                })
                .collect(),
        };

        let mut pulse_url = self.config.api_url.clone();
        pulse_url.set_path("/api/my/pulses");

        match self.send_pulse_internal(&pulse).await {
            Ok(()) => {
                self.client
                    .log_message(MessageType::INFO, "XP pulse sent successfully")
                    .await;
            }
            Err(err) => {
                self.pulse_cache.save(&pulse).ok();

                self.client
                    .log_message(MessageType::ERROR, format!("Error sending XP pulse: {err}"))
                    .await;
            }
        }

        xp_gained_by_language.clear();
    }

    async fn send_pulse_internal(&self, pulse: &Pulse) -> Result<()> {
        let mut pulse_url = self.config.api_url.clone();
        pulse_url.set_path("/api/my/pulses");

        self.http_client
            .post(pulse_url)
            .timeout(Duration::from_secs(10))
            .header("User-Agent", self.user_agent().await)
            .header("X-API-Token", &self.config.api_token)
            .json(&pulse)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for CodeStatsLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        *self.client_info.write().await = params.client_info;

        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: self.name().to_string(),
                version: Some(self.version().to_string()),
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

    async fn shutdown(&self) -> jsonrpc::Result<()> {
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
        let total_xp_gained = xp_gained_by_language.entry(language).or_insert(0);
        *total_xp_gained += xp_gained;

        self.pulse_tx.send(()).await.ok();
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {}

#[tokio::main]
async fn main() -> Result<()> {
    let _cli = Cli::parse();

    let config = Config::read()?;

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (pulse_tx, mut pulse_rx) = mpsc::channel::<()>(100);
    let pulse_cache = PulseCache::new()?;

    let (service, socket) = LspService::new({
        let pulse_tx = pulse_tx.clone();
        |client| {
            Arc::new(CodeStatsLanguageServer::new(
                client,
                config,
                pulse_tx,
                pulse_cache,
            ))
        }
    });

    // Spawn a task to periodically flush any pending XP in the queue.
    tokio::spawn({
        let pulse_tx = pulse_tx.clone();
        async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                pulse_tx.send(()).await.ok();
            }
        }
    });

    // Spawn a task to periodically send any cached pulses.
    tokio::spawn({
        let server = service.inner().clone();
        async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;

                if let Err(err) = server.send_cached_pulses().await {
                    server
                        .client
                        .log_message(
                            MessageType::ERROR,
                            format!("Error sending cached XP pulses: {err}"),
                        )
                        .await;
                }
            }
        }
    });

    tokio::spawn({
        let server = service.inner().clone();
        async move {
            let mut last_pulse_at = Instant::now();
            let debounce_duration = Duration::from_secs(10);

            while pulse_rx.recv().await.is_some() {
                if last_pulse_at.elapsed() >= debounce_duration {
                    server.send_pulse().await;
                    last_pulse_at = Instant::now();
                }
            }
        }
    });

    Server::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}
