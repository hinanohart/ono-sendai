//! Application state + event loop.

use std::io;
use std::time::Duration;

use anyhow::Result;
use ratatui::backend::Backend;
use ratatui::Terminal;

use crate::event::{Event, EventStream};
use crate::ui;

#[derive(Debug)]
pub struct App {
    pub input: String,
    pub log: Vec<String>,
    pub should_quit: bool,
    pub status: String,
}

impl App {
    #[must_use]
    pub fn new() -> Self {
        Self {
            input: String::new(),
            log: vec!["// ono-sendai online. type `:q` or press Ctrl-C to exit.".into()],
            should_quit: false,
            status: format!("v{}", env!("CARGO_PKG_VERSION")),
        }
    }

    pub fn handle_input_key(&mut self, c: char) {
        self.input.push(c);
    }

    pub fn handle_backspace(&mut self) {
        self.input.pop();
    }

    pub fn handle_enter(&mut self) {
        if self.input == ":q" {
            self.should_quit = true;
            return;
        }
        if self.input.is_empty() {
            return;
        }
        self.log.push(format!("> {}", self.input));
        self.log
            .push("  (Phase 1 stub: orchestrator not wired yet)".into());
        self.input.clear();
    }

    pub async fn run<B: Backend + io::Write>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        let mut events = EventStream::new(Duration::from_millis(16));
        while !self.should_quit {
            terminal.draw(|f| ui::draw(f, self))?;
            match events.next().await {
                Some(Event::Key(c)) => self.handle_input_key(c),
                Some(Event::Enter) => self.handle_enter(),
                Some(Event::Backspace) => self.handle_backspace(),
                Some(Event::Quit) => self.should_quit = true,
                Some(Event::Tick) | None => {}
            }
        }
        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_pushes_to_log_and_clears_input() {
        let mut app = App::new();
        let baseline = app.log.len();
        app.input = "hello".into();
        app.handle_enter();
        assert!(app.input.is_empty());
        assert_eq!(app.log.len(), baseline + 2);
        assert!(app.log[baseline].contains("hello"));
    }

    #[test]
    fn colon_q_quits() {
        let mut app = App::new();
        app.input = ":q".into();
        app.handle_enter();
        assert!(app.should_quit);
    }
}
