//! ono-sendai — Console Cowboy deck
//!
//! Single-binary terminal cyberdeck: TUI (ratatui) + local LLM (ollama/llama.cpp via trait)
//! + MCP client (rmcp) + age-encrypted local context store + seccomp/landlock sandbox
//! for untrusted MCP servers. No telemetry, no cloud dependency, offline-first.

use anyhow::Result;
use clap::{Parser, Subcommand};
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
    let cli = Cli::parse();
    init_tracing(cli.verbose);
    info!(version = env!("CARGO_PKG_VERSION"), "ono-sendai starting");

    match cli.command.unwrap_or(Command::Run) {
        Command::Run => run_tui(cli.config).await,
        Command::Config => print_config(cli.config),
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

async fn run_tui(_config: Option<std::path::PathBuf>) -> Result<()> {
    deck_tui::run().await
}

fn print_config(_config: Option<std::path::PathBuf>) -> Result<()> {
    let cfg = deck_core::Config::default();
    println!("{cfg:#?}");
    Ok(())
}

async fn run_doctor() -> Result<()> {
    println!("ono-sendai doctor");
    println!("  version    : {}", env!("CARGO_PKG_VERSION"));
    println!("  sandbox    : {}", deck_sandbox::availability());
    println!("  llm        : ollama default endpoint http://127.0.0.1:11434");
    println!("  mcp        : stdio transport (rmcp)");
    println!("  store      : sqlite (bundled) + age");
    Ok(())
}

async fn handle_deck(action: DeckAction) -> Result<()> {
    match action {
        DeckAction::New { name } => {
            println!("(stub) create deck: {name}");
            Ok(())
        }
        DeckAction::List => {
            println!("(stub) list decks");
            Ok(())
        }
    }
}
