use crate::models::{Message, ProcessingStatus};
use rusqlite::{Connection, Result, params};
use std::path::Path;
use std::sync::Mutex;

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn new(path: &str) -> Result<Self> {
        // Create the parent directory if it doesn't exist (e.g., /dev/shm/tokenmin/)
        if path != ":memory:"
            && let Some(parent) = Path::new(path).parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|e| {
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                    Some(format!(
                        "Failed to create parent directory '{}': {}",
                        parent.display(),
                        e
                    )),
                )
            })?;
        }

        let conn = Connection::open(path).map_err(|e| {
            eprintln!("Failed to open database at {}: {}", path, e);
            e
        })?;

        // Enable WAL mode for better concurrency
        conn.pragma_update(None, "journal_mode", "WAL")?;

        // Configure a busy timeout so concurrent writes retry instead of immediately failing with SQLITE_BUSY
        conn.busy_timeout(std::time::Duration::from_millis(5000))?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_schema()?;
        Ok(db)
    }

    /// Acquire the connection lock, converting a poisoned Mutex into a rusqlite error
    /// instead of panicking, so a single task failure won't bring down the daemon.
    fn lock_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|e| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_MISUSE),
                Some(format!("Mutex poisoned: {}", e)),
            )
        })
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.lock_conn()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                raw_content TEXT NOT NULL,
                processed_content TEXT,
                status TEXT NOT NULL,
                model TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER
            )",
            [],
        )?;
        Ok(())
    }

    // TODO: Disallow dead_code once the client application is integrated and using these helpers.
    #[allow(dead_code)]
    pub fn insert_message(&self, message: &Message) -> Result<i64> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT INTO messages (session_id, role, raw_content, status, model, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                message.session_id,
                message.role,
                message.raw_content,
                message.status,
                message.model,
                now,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Atomically claim all pending messages by transitioning them to `processing`
    /// within a single timestamp so concurrent watcher instances cannot double-process
    /// the same rows.
    pub fn poll_pending_messages(&self) -> Result<Vec<Message>> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let conn = self.lock_conn()?;

        // Atomically claim all currently pending messages.
        conn.execute(
            "UPDATE messages SET status = 'processing', updated_at = ?1 WHERE status = 'pending'",
            params![now],
        )?;

        // Return only the messages claimed in this call (identified by their updated_at timestamp).
        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, raw_content, processed_content, status, model
             FROM messages
             WHERE status = 'processing' AND updated_at = ?1
             ORDER BY created_at ASC",
        )?;

        let message_iter = stmt.query_map(params![now], |row| {
            Ok(Message {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                raw_content: row.get(3)?,
                processed_content: row.get(4)?,
                status: row.get(5)?,
                model: row.get(6)?,
            })
        })?;

        let mut messages = Vec::new();
        for message in message_iter {
            messages.push(message?);
        }
        Ok(messages)
    }

    pub fn update_message(
        &self,
        id: i64,
        status: ProcessingStatus,
        processed_content: Option<String>,
    ) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let conn = self.lock_conn()?;
        conn.execute(
            "UPDATE messages SET status = ?1, processed_content = ?2, updated_at = ?3 WHERE id = ?4",
            params![status, processed_content, now, id],
        )?;
        Ok(())
    }

    // TODO: Disallow dead_code once the client application is integrated and using these helpers.
    #[allow(dead_code)]
    pub fn get_message_by_id(&self, id: i64) -> Result<Message> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, raw_content, processed_content, status, model FROM messages WHERE id = ?1"
        )?;

        stmt.query_row(params![id], |row| {
            Ok(Message {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                raw_content: row.get(3)?,
                processed_content: row.get(4)?,
                status: row.get(5)?,
                model: row.get(6)?,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Message, ProcessingStatus};

    #[test]
    fn test_db_schema_and_ops() {
        // Use in-memory DB for tests
        let db = Db::new(":memory:").unwrap();

        let msg = Message {
            id: 0,
            session_id: "session-1".into(),
            role: "user".into(),
            raw_content: "Code: ```rust\nfn main() {}\n```".into(),
            processed_content: None,
            status: ProcessingStatus::Pending,
            model: Some("gpt-4".into()),
        };

        let id = db.insert_message(&msg).unwrap();
        assert!(id > 0);

        // poll_pending_messages atomically claims messages as 'processing'
        let claimed = db.poll_pending_messages().unwrap();
        assert_eq!(claimed.len(), 1);
        assert_eq!(claimed[0].id, id);
        assert_eq!(claimed[0].status, ProcessingStatus::Processing);

        db.update_message(
            id,
            ProcessingStatus::Completed,
            Some("Optimized content".into()),
        )
        .unwrap();

        let pending_after = db.poll_pending_messages().unwrap();
        assert_eq!(pending_after.len(), 0);

        // Verify update
        let msg_after = db.get_message_by_id(id).unwrap();
        assert_eq!(msg_after.status, ProcessingStatus::Completed);
        assert_eq!(msg_after.processed_content.unwrap(), "Optimized content");
    }

    #[test]
    fn test_db_invalid_status_handling() {
        let db = Db::new(":memory:").unwrap();
        let msg = Message {
            id: 0,
            session_id: "session-1".into(),
            role: "user".into(),
            raw_content: "test".into(),
            processed_content: None,
            status: ProcessingStatus::Pending,
            model: None,
        };
        let id = db.insert_message(&msg).unwrap();

        // Manually corrupt the status in the DB
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "UPDATE messages SET status = 'corrupt' WHERE id = ?1",
            params![id],
        )
        .unwrap();
        drop(conn);

        let result = db.get_message_by_id(id);
        assert!(result.is_err());
    }
}
