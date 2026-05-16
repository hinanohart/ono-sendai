//! deck-store — encrypted, durable session store.
//!
//! Phase 1 ships a [`SqliteStore`] backed by a plain (unencrypted) `SQLite`
//! file. `age`-encrypted-at-rest is implemented via a "decrypt to tmpfs on
//! open, re-encrypt on close" lifecycle in Phase 2; the on-disk layout is
//! already final and lives at `$XDG_DATA_HOME/ono-sendai/decks/<id>/`.

pub mod sqlite;

pub use sqlite::SqliteStore;
