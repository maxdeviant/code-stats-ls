mod cache;
mod config;
mod hook;
mod languages;
mod pulse;

use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::Local;
use clap::{Parser, Subcommand};
use tokio::sync::{mpsc, Mutex, RwLock};
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::cache::PulseCache;
use crate::config::Config;
use crate::languages::language_for_extension;
use crate::pulse::{Pulse, PulseSender, PulseXp};

struct CodeStatsLanguageServer {
    client: Client,
    pulse_sender: PulseSender,
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
            pulse_sender: PulseSender::new(config, Duration::from_secs(10)),
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

        language_for_extension(extension).map(|language| language.to_string())
    }

    async fn send_cached_pulses(&self) -> Result<()> {
        // Take the pulses out of the cache (rather than just listing them) so
        // that other processes sharing the cache (e.g., the Claude Code hook)
        // don't send them too.
        let pulses = self.pulse_cache.take(usize::MAX)?;
        let user_agent = self.user_agent().await;

        let mut sent_count = 0;

        for pulse in pulses {
            match self.pulse_sender.send(&pulse, &user_agent).await {
                Ok(()) => {
                    sent_count += 1;
                }
                Err(err) => {
                    self.pulse_cache.save(&pulse)?;

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

        let user_agent = self.user_agent().await;

        match self.pulse_sender.send(&pulse, &user_agent).await {
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
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Records XP from a Claude Code `PostToolUse` hook.
    ///
    /// Reads the hook input JSON from stdin.
    Hook,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let config = Config::read()?;

    match cli.command {
        Some(Command::Hook) => hook::run(config).await,
        None => run_language_server(config).await,
    }
}

async fn run_language_server(config: Config) -> Result<()> {
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
