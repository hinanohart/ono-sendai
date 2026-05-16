//! crossterm → app event bridging. Polls in a blocking thread, forwards
//! decoded events through an async channel so the main loop stays cleanly
//! tokio-friendly.

use std::time::Duration;

use crossterm::event::{self as ct, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

#[derive(Debug, Clone)]
pub enum Event {
    Key(char),
    Enter,
    Backspace,
    Quit,
    Tick,
}

pub struct EventStream {
    rx: UnboundedReceiver<Event>,
}

impl std::fmt::Debug for EventStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventStream").finish_non_exhaustive()
    }
}

impl EventStream {
    pub fn new(tick: Duration) -> Self {
        let (tx, rx) = unbounded_channel();
        let tx_tick = tx.clone();
        std::thread::spawn(move || loop {
            if ct::poll(tick).unwrap_or(false) {
                if let Ok(ev) = ct::read() {
                    if let ct::Event::Key(KeyEvent {
                        code, modifiers, ..
                    }) = ev
                    {
                        let mapped = map_key(code, modifiers);
                        if tx.send(mapped).is_err() {
                            return;
                        }
                    }
                }
            } else if tx_tick.send(Event::Tick).is_err() {
                return;
            }
        });
        Self { rx }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}

fn map_key(code: KeyCode, mods: KeyModifiers) -> Event {
    if mods.contains(KeyModifiers::CONTROL) && matches!(code, KeyCode::Char('c' | 'd')) {
        return Event::Quit;
    }
    match code {
        KeyCode::Enter => Event::Enter,
        KeyCode::Backspace => Event::Backspace,
        KeyCode::Esc => Event::Quit,
        KeyCode::Char(c) => Event::Key(c),
        _ => Event::Tick,
    }
}
