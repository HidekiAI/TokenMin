use rusqlite::{params, Connection, Result};
use crate::models::{Message, ProcessingStatus};

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        // Enable WAL mode for better concurrency
        conn.pragma_update(None, "journal_mode", "WAL")?;
        
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute(
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

    pub fn insert_message(&self, message: &Message) -> Result<i64> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        self.conn.execute(
            "INSERT INTO messages (session_id, role, raw_content, status, model, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                message.session_id,
                message.role,
                message.raw_content,
                serde_json::to_string(&message.status).unwrap().trim_matches('"'),
                message.model,
                now,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn poll_pending_messages(&self) -> Result<Vec<Message>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, role, raw_content, processed_content, status, model FROM messages WHERE status = 'pending' ORDER BY created_at ASC"
        )?;
        
        let message_iter = stmt.query_map([], |row| {
            let status_str: String = row.get(5)?;
            let status: ProcessingStatus = serde_json::from_str(&format!("\"{}\"", status_str)).unwrap();
            
            Ok(Message {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                raw_content: row.get(3)?,
                processed_content: row.get(4)?,
                status,
                model: row.get(6)?,
            })
        })?;

        let mut messages = Vec::new();
        for message in message_iter {
            messages.push(message?);
        }
        Ok(messages)
    }

    pub fn update_message(&self, id: i64, status: ProcessingStatus, processed_content: Option<String>) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
            
        let status_str = serde_json::to_string(&status).unwrap().trim_matches('"').to_string();

        self.conn.execute(
            "UPDATE messages SET status = ?1, processed_content = ?2, updated_at = ?3 WHERE id = ?4",
            params![status_str, processed_content, now, id],
        )?;
        Ok(())
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

        let pending = db.poll_pending_messages().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, id);
        assert_eq!(pending[0].status, ProcessingStatus::Pending);

        db.update_message(id, ProcessingStatus::Completed, Some("Optimized content".into())).unwrap();
        
        let pending_after = db.poll_pending_messages().unwrap();
        assert_eq!(pending_after.len(), 0);
        
        // Verify update
        let mut stmt = db.conn.prepare("SELECT status, processed_content FROM messages WHERE id = ?1").unwrap();
        let (status_str, processed): (String, Option<String>) = stmt.query_row(params![id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        }).unwrap();
        
        assert_eq!(status_str, "completed");
        assert_eq!(processed.unwrap(), "Optimized content");
    }
}
