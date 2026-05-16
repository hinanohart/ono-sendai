//! deck-tui — ratatui front-end.
//!
//! The TUI owns the terminal and an [`App`] that drives input + render.
//! All LLM/MCP/Store traffic flows through a [`deck_orchestrator::Handle`].

mod app;
mod event;
mod ui;

use std::io;

use anyhow::Result;
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

async fn run_with(app: &mut App) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = app.run(&mut terminal).await;
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    result
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
