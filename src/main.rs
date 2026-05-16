//! ono-sendai — Console Cowboy deck
//!
//! Single-binary terminal cyberdeck: TUI (ratatui) + local LLM (ollama/mock)
//! + MCP client (rmcp) + age-encrypted local context store + seccomp/landlock
//! sandbox for untrusted MCP servers. No telemetry, no cloud dependency.

use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use deck_core::{LlmBackend, SessionId, Store};
use deck_orchestrator::Runtime;
use deck_store::SqliteStore;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "ono-sendai", version, about = "Console Cowboy deck")]
struct Cli {
    /// Path to config file (default: $XDG_CONFIG_HOME/ono-sendai/config.toml)
    #[arg(long, global = true)]
    config: Option<std::path::PathBuf>,

    /// Verbosity (-v info, -vv debug, -vvv trace)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Force a specific LLM backend (overrides config). "mock" runs offline.
    #[arg(long, global = true)]
    backend: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Launch the TUI (default if no subcommand)
    Run,
    /// Print active config (resolved from env > cli > file > default)
    Config,
    /// Show backend / sandbox / store diagnostics
    Doctor,
    /// Manage decks (encrypted context vaults)
    Deck {
        #[command(subcommand)]
        action: DeckAction,
    },
}

#[derive(Subcommand, Debug)]
enum DeckAction {
    /// Create a new encrypted deck
    New { name: String },
    /// List known decks
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut cli = Cli::parse();
    init_tracing(cli.verbose);
    info!(version = env!("CARGO_PKG_VERSION"), "ono-sendai starting");

    let backend_override = cli.backend.take();
    let cmd = cli.command.take().unwrap_or(Command::Run);
    match cmd {
        Command::Run => run_tui(backend_override).await,
        Command::Config => print_config(),
        Command::Doctor => run_doctor().await,
        Command::Deck { action } => handle_deck(action).await,
    }
}

fn init_tracing(verbose: u8) {
    let level = match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("ono_sendai={level},deck_={level}")));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}

fn data_root() -> Result<std::path::PathBuf> {
    let base = dirs::data_dir()
        .ok_or_else(|| anyhow::anyhow!("could not resolve XDG_DATA_HOME"))?
        .join("ono-sendai");
    std::fs::create_dir_all(&base).context("create data root")?;
    Ok(base)
}

fn build_store() -> Result<Arc<dyn Store>> {
    let path = data_root()?.join("default").join("sessions.db");
    let store = SqliteStore::open(&path).context("open store")?;
    Ok(Arc::new(store))
}

fn build_llm(cfg: &deck_core::config::LlmConfig) -> Result<Arc<dyn LlmBackend>> {
    let backend: Box<dyn LlmBackend> = deck_llm::from_config(cfg).context("build llm")?;
    Ok(Arc::from(backend))
}

async fn run_tui(backend_override: Option<String>) -> Result<()> {
    let mut cfg = deck_core::Config::default();
    if let Some(b) = backend_override {
        cfg.llm.backend = b;
    }
    let store = build_store()?;
    let llm = build_llm(&cfg.llm)?;
    let runtime = Runtime::spawn(llm, store, cfg.llm.model.clone());
    let session = SessionId::new();
    let result = deck_tui::run_with_handle(runtime.handle.clone(), session).await;
    runtime.shutdown().await;
    result
}

fn print_config() -> Result<()> {
    let cfg = deck_core::Config::default();
    println!("{cfg:#?}");
    Ok(())
}

async fn run_doctor() -> Result<()> {
    println!("ono-sendai doctor");
    println!("  version    : {}", env!("CARGO_PKG_VERSION"));
    println!("  sandbox    : {}", deck_sandbox::availability());
    println!("  llm        : ollama default endpoint http://127.0.0.1:11434");
    println!("  mcp        : stdio transport");
    println!("  store      : {}", data_root()?.display());
    Ok(())
}

async fn handle_deck(action: DeckAction) -> Result<()> {
    match action {
        DeckAction::New { name } => {
            let dir = data_root()?.join(&name);
            std::fs::create_dir_all(&dir)?;
            println!("created deck at {}", dir.display());
            Ok(())
        }
        DeckAction::List => {
            let root = data_root()?;
            for entry in std::fs::read_dir(&root)? {
                let entry = entry?;
                if entry.path().is_dir() {
                    println!("{}", entry.file_name().to_string_lossy());
                }
            }
            Ok(())
        }
    }
}
