//! Plain SQLite store, designed to be wrapped by an `age`-encrypted file in
//! Phase 2 (decrypt-to-tmpfs lifecycle). The schema is intentionally small:
//! a single `messages` table keyed by `(session_id, seq)`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use deck_core::{DeckError, Message, Result, SessionId, Store};
use rusqlite::{params, Connection};

#[derive(Debug, Clone)]
pub struct SqliteStore {
    conn: Arc<Mutex<Connection>>,
    path: PathBuf,
}

impl SqliteStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path).map_err(|e| DeckError::Store(format!("open: {e}")))?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS sessions (
                id     TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
            );
            CREATE TABLE IF NOT EXISTS messages (
                session_id TEXT NOT NULL REFERENCES sessions(id),
                seq        INTEGER NOT NULL,
                role       TEXT NOT NULL,
                content    TEXT NOT NULL,
                tool_calls TEXT NOT NULL DEFAULT '[]',
                PRIMARY KEY (session_id, seq)
            );
            "#,
        )
        .map_err(|e| DeckError::Store(format!("init schema: {e}")))?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            path,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[async_trait]
impl Store for SqliteStore {
    async fn append(&self, session: SessionId, msg: &Message) -> Result<()> {
        let role = match msg.role {
            deck_core::Role::System => "system",
            deck_core::Role::User => "user",
            deck_core::Role::Assistant => "assistant",
            deck_core::Role::Tool => "tool",
        };
        let tool_calls = serde_json::to_string(&msg.tool_calls)?;
        let content = msg.content.clone();
        let session_str = session.to_string();
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = conn.lock().expect("store mutex");
            conn.execute(
                "INSERT OR IGNORE INTO sessions(id) VALUES (?1)",
                params![session_str],
            )
            .map_err(|e| DeckError::Store(format!("upsert session: {e}")))?;
            let next: i64 = conn
                .query_row(
                    "SELECT COALESCE(MAX(seq), -1) + 1 FROM messages WHERE session_id = ?1",
                    params![session_str],
                    |row| row.get(0),
                )
                .map_err(|e| DeckError::Store(format!("next seq: {e}")))?;
            conn.execute(
                "INSERT INTO messages(session_id, seq, role, content, tool_calls) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![session_str, next, role, content, tool_calls],
            )
            .map_err(|e| DeckError::Store(format!("insert message: {e}")))?;
            Ok(())
        })
        .await
        .map_err(|e| DeckError::Store(format!("join: {e}")))?
    }

    async fn load(&self, session: SessionId) -> Result<Vec<Message>> {
        let session_str = session.to_string();
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<Message>> {
            let conn = conn.lock().expect("store mutex");
            let mut stmt = conn
                .prepare(
                    "SELECT role, content, tool_calls FROM messages WHERE session_id = ?1 ORDER BY seq ASC",
                )
                .map_err(|e| DeckError::Store(format!("prepare: {e}")))?;
            let rows = stmt
                .query_map(params![session_str], |row| {
                    let role: String = row.get(0)?;
                    let content: String = row.get(1)?;
                    let tool_calls: String = row.get(2)?;
                    Ok((role, content, tool_calls))
                })
                .map_err(|e| DeckError::Store(format!("query: {e}")))?;
            let mut out = Vec::new();
            for r in rows {
                let (role, content, tc) = r.map_err(|e| DeckError::Store(format!("row: {e}")))?;
                let role = match role.as_str() {
                    "system" => deck_core::Role::System,
                    "user" => deck_core::Role::User,
                    "tool" => deck_core::Role::Tool,
                    _ => deck_core::Role::Assistant,
                };
                let tool_calls = serde_json::from_str(&tc).unwrap_or_default();
                out.push(Message {
                    role,
                    content,
                    tool_calls,
                });
            }
            Ok(out)
        })
        .await
        .map_err(|e| DeckError::Store(format!("join: {e}")))?
    }

    async fn list(&self) -> Result<Vec<SessionId>> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<SessionId>> {
            let conn = conn.lock().expect("store mutex");
            let mut stmt = conn
                .prepare("SELECT id FROM sessions ORDER BY created_at ASC")
                .map_err(|e| DeckError::Store(format!("prepare: {e}")))?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| DeckError::Store(format!("query: {e}")))?;
            let mut out = Vec::new();
            for r in rows {
                let s = r.map_err(|e| DeckError::Store(format!("row: {e}")))?;
                let uuid = uuid::Uuid::parse_str(&s)
                    .map_err(|e| DeckError::Store(format!("uuid: {e}")))?;
                out.push(SessionId(uuid));
            }
            Ok(out)
        })
        .await
        .map_err(|e| DeckError::Store(format!("join: {e}")))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_core::Role;
    use tempfile::TempDir;

    #[tokio::test]
    async fn append_and_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("sessions.db");
        let store = SqliteStore::open(&path).expect("open");
        let session = SessionId::new();
        store
            .append(
                session,
                &Message {
                    role: Role::User,
                    content: "hi".into(),
                    tool_calls: vec![],
                },
            )
            .await
            .unwrap();
        let msgs = store.load(session).await.unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "hi");
    }
}
