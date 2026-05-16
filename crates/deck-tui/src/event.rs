//! crossterm → app event bridging.
//!
//! Polls in a dedicated OS thread (crossterm's API is blocking) and
//! forwards decoded events through an unbounded channel so the tokio main
//! loop stays cleanly async. Unhandled key codes (arrows, F-keys, etc.)
//! do NOT emit a `Tick`; only the poll timeout does. This keeps the tick
//! channel a reliable heartbeat instead of an event-keyed noise source.
//!
//! `EventStream` owns the poll thread via an `AtomicBool` shutdown flag
//! and joins it on drop, so reconstructing the stream (e.g. across a
//! suspend/resume) does not leak threads.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
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
    shutdown: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for EventStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventStream").finish_non_exhaustive()
    }
}

impl EventStream {
    pub fn new(tick: Duration) -> Self {
        let (tx, rx) = unbounded_channel();
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_thread = shutdown.clone();
        let join = std::thread::spawn(move || {
            while !shutdown_thread.load(Ordering::Relaxed) {
                if ct::poll(tick).unwrap_or(false) {
                    if let Ok(ct::Event::Key(KeyEvent {
                        code, modifiers, ..
                    })) = ct::read()
                    {
                        if let Some(ev) = map_key(code, modifiers) {
                            if tx.send(ev).is_err() {
                                return;
                            }
                        }
                    }
                } else if tx.send(Event::Tick).is_err() {
                    return;
                }
            }
        });
        Self {
            rx,
            shutdown,
            join: Some(join),
        }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}

impl Drop for EventStream {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

/// Map a crossterm key code into an [`Event`]. Returns `None` for keys
/// the TUI does not care about (so the channel is not flooded with
/// pseudo-ticks indistinguishable from idle ticks).
const fn map_key(code: KeyCode, mods: KeyModifiers) -> Option<Event> {
    if mods.contains(KeyModifiers::CONTROL) && matches!(code, KeyCode::Char('c' | 'd')) {
        return Some(Event::Quit);
    }
    match code {
        KeyCode::Enter => Some(Event::Enter),
        KeyCode::Backspace => Some(Event::Backspace),
        KeyCode::Esc => Some(Event::Quit),
        KeyCode::Char(c) => Some(Event::Key(c)),
        _ => None,
    }
}
