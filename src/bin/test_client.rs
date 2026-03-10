use hmac::{Hmac, Mac};
use rusqlite::{Connection, params};
use sha2::Sha256;
use std::env;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

fn compute_checksum(
    secret: &str,
    session_id: &str,
    role: &str,
    model: Option<&str>,
    text: &str,
) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(&(session_id.len() as u32).to_le_bytes());
    mac.update(session_id.as_bytes());
    mac.update(&(role.len() as u32).to_le_bytes());
    mac.update(role.as_bytes());
    if let Some(m) = model {
        mac.update(&[1]);
        mac.update(&(m.len() as u32).to_le_bytes());
        mac.update(m.as_bytes());
    } else {
        mac.update(&[0]);
    }
    mac.update(&(text.len() as u32).to_le_bytes());
    mac.update(text.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn poll_for_status(
    conn: &Connection,
    msg_id: i64,
    expected_status: &str,
    timeout: u64,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    for _ in 0..timeout {
        let mut stmt =
            conn.prepare("SELECT status, processed_content FROM messages WHERE id = ?1")?;
        let row_result = stmt.query_row(params![msg_id], |row| {
            let status: String = row.get(0)?;
            let processed_content: Option<String> = row.get(1)?;
            Ok((status, processed_content))
        });

        if let Ok((status, processed_content)) = row_result {
            if status == expected_status {
                println!("\n✅ Message processed successfully! Status: {}", status);
                return Ok(processed_content);
            } else if status == "failed"
                || (expected_status == "completed" && status == "skipped")
                || (expected_status == "skipped" && status == "completed")
            {
                println!(
                    "\n❌ Message processing finished with unexpected status: {}",
                    status
                );
                return Ok(processed_content);
            }
            println!("Status is '{}', waiting...", status);
        }
        thread::sleep(Duration::from_secs(1));
    }
    Err("Timeout waiting for message to process.".into())
}

fn main() {
    let db_path = env::var("TOKENMIN_DB")
        .unwrap_or_else(|_| "/dev/shm/chat_and_plan/message_queue.sqlite3".to_string());
    let hmac_secret =
        env::var("TOKENMIN_HMAC_SECRET").unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());

    thread::sleep(Duration::from_secs(1));
    if !std::path::Path::new(&db_path).exists() {
        eprintln!(
            "Database {} does not exist yet. Did the daemon start correctly?",
            db_path
        );
        std::process::exit(1);
    }

    let conn = Connection::open(&db_path).expect("Failed to open database");

    let raw_text = "Hey there! I was just wondering if you could help me out. This is just a lot of conversational filler to see if the model actually summarizes it down.\nAnyway, here is the core logic:\n```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```\nThanks so much for taking a look at this. It really helps a lot.";

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    // Test 1: Full Compaction
    let checksum1 = compute_checksum(
        &hmac_secret,
        "test_session",
        "user",
        Some("gpt-4"),
        raw_text,
    );
    conn.execute(
        "INSERT INTO messages (session_id, role, raw_content, status, model, hmac_signature, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params!["test_session", "user", raw_text, "pending", "gpt-4", checksum1, now_ms],
    ).expect("Failed to insert Test 1");

    let msg_id = conn.last_insert_rowid();
    println!(
        "[Test 1] Inserted message for compaction with ID: {}",
        msg_id
    );

    match poll_for_status(&conn, msg_id, "completed", 20) {
        Ok(content) => {
            println!("Original Length: {}", raw_text.len());
            let p_len = content.as_ref().map(|s| s.len()).unwrap_or(0);
            println!("Processed Length: {}", p_len);
            println!(
                "Processed Content:\n------------------------------\n{}\n------------------------------",
                content.unwrap_or_default()
            );
        }
        Err(e) => println!("{}", e),
    }

    // Test 2: Compaction Bypass
    let bypass_text = "Bypass this! No changes needed.";
    let checksum2 = compute_checksum(
        &hmac_secret,
        "test_session2",
        "user",
        Some("copilot-chat"),
        bypass_text,
    );
    conn.execute(
        "INSERT INTO messages (session_id, role, raw_content, status, model, hmac_signature, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params!["test_session2", "user", bypass_text, "pending", "copilot-chat", checksum2, now_ms + 1],
    ).expect("Failed to insert Test 2");

    let bypass_id = conn.last_insert_rowid();
    println!("\n[Test 2] Inserted bypass message with ID: {}", bypass_id);

    match poll_for_status(&conn, bypass_id, "skipped", 20) {
        Ok(content) => println!("Processed Content: {}", content.unwrap_or_default()),
        Err(e) => println!("{}", e),
    }
}
