//! deck-tui — ratatui front-end.
//!
//! The TUI owns the terminal and an [`App`] that drives input + render.
//! All LLM/MCP/Store traffic flows through a [`deck_orchestrator::Handle`].

mod app;
mod event;
mod ui;

use std::io::{self, IsTerminal, Stdout};

use anyhow::{bail, Context, Result};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use deck_orchestrator::Handle;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

pub use app::App;

/// Standalone entry (no orchestrator wired). Useful for `--no-llm` smoke
/// tests and the first-launch onboarding screen.
pub async fn run() -> Result<()> {
    let mut app = App::new(None);
    run_with(&mut app).await
}

/// Wired entry. The binary crate calls this with an Orchestrator handle.
pub async fn run_with_handle(handle: Handle, session: deck_core::SessionId) -> Result<()> {
    let mut app = App::new(Some(AppHandle { handle, session }));
    run_with(&mut app).await
}

/// RAII guard for the terminal alt-screen + raw-mode pair. Whatever the
/// inner future does (Ok, Err, panic), the terminal is restored before we
/// leave this scope. Without this guard, an early error left the user's
/// shell in raw-mode + alt-screen.
struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    fn enter() -> Result<Self> {
        if !io::stdin().is_terminal() {
            bail!(
                "ono-sendai needs an interactive TTY on stdin. \
                 Run it from a real terminal — not a pipe, CI runner, or \
                 non-PTY ssh session."
            );
        }
        enable_raw_mode().context("enable raw mode")?;
        let mut stdout = io::stdout();
        if let Err(e) = execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
            // We entered raw mode but failed to enter alt-screen; undo
            // the raw-mode so the shell is left usable.
            let _ = disable_raw_mode();
            return Err(e).context("enter alternate screen");
        }
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).context("create terminal")?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Best-effort restore. Don't panic from Drop and don't shadow the
        // caller's error with a teardown failure.
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
        let _ = disable_raw_mode();
    }
}

async fn run_with(app: &mut App) -> Result<()> {
    let mut guard = TerminalGuard::enter()?;
    app.run(&mut guard.terminal).await
}

/// Carrier struct so `App` can hold an optional orchestrator binding
/// without `App::new` taking three arguments.
#[derive(Clone)]
pub struct AppHandle {
    pub handle: Handle,
    pub session: deck_core::SessionId,
}

impl std::fmt::Debug for AppHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppHandle")
            .field("session", &self.session)
            .finish_non_exhaustive()
    }
}
