mod config;
mod db;
mod engine;
mod models;

use crate::config::Config;
use crate::db::Db;
use crate::engine::Engine;
use crate::engine::sanctuary::{extract_code_blocks, restore_code_blocks};
use crate::engine::summarizer::Summarizer;
use crate::models::{Message, ProcessingStatus};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("TokenMin started.");

    let config = Config::from_env();
    let db = Arc::new(Db::new(&config.db_path)?);
    let engine = Engine::new(config.bypass_models.clone());
    let hmac_secret = Arc::new(config.hmac_secret.clone());

    // Summarizer::new now returns a Result
    let summarizer = Summarizer::new(config.ollama_url.clone(), config.ollama_model.clone())
        .map_err(|e| std::io::Error::other(format!("Failed to initialize summarizer: {}", e)))?;

    println!(
        "Polling for messages in: {} (Interval: {}ms)",
        config.db_path, config.poll_interval_ms
    );

    loop {
        let db_clone = Arc::clone(&db);
        let poll_handle =
            tokio::task::spawn_blocking(move || db_clone.poll_pending_messages()).await;

        match poll_handle {
            Ok(Ok(messages)) => {
                for msg in messages {
                    process_message(
                        Arc::clone(&db),
                        &engine,
                        &summarizer,
                        msg,
                        Arc::clone(&hmac_secret),
                    )
                    .await;
                }
            }
            Ok(Err(e)) => eprintln!("Database poll error: {}", e),
            Err(e) => eprintln!("Database task failed (panic or cancelled): {}", e),
        }
        sleep(Duration::from_millis(config.poll_interval_ms)).await;
    }
}

async fn process_message(
    db: Arc<Db>,
    engine: &Engine,
    summarizer: &Summarizer,
    msg: Message,
    hmac_secret: Arc<String>,
) {
    let id = msg.id;
    println!("Processing message ID: {}", id);

    // 0. Cryptographic HMAC Verification
    let mut mac = match HmacSha256::new_from_slice(hmac_secret.as_bytes()) {
        Ok(mac) => mac,
        Err(_) => {
            eprintln!("SECURITY ERROR: Invalid HMAC secret key length");
            return;
        }
    };
    mac.update(&(msg.session_id.len() as u32).to_le_bytes());
    mac.update(msg.session_id.as_bytes());
    mac.update(&(msg.role.len() as u32).to_le_bytes());
    mac.update(msg.role.as_bytes());
    if let Some(model) = &msg.model {
        mac.update(&[1]);
        mac.update(&(model.len() as u32).to_le_bytes());
        mac.update(model.as_bytes());
    } else {
        mac.update(&[0]);
    }
    mac.update(&(msg.raw_content.len() as u32).to_le_bytes());
    mac.update(msg.raw_content.as_bytes());
    let is_valid = hex::decode(&msg.hmac_signature)
        .map(|expected_mac| mac.verify_slice(&expected_mac).is_ok())
        .unwrap_or(false);

    if !is_valid {
        eprintln!("SECURITY ERROR: HMAC mismatch for message {}", id);
        let db_clone = Arc::clone(&db);
        match tokio::task::spawn_blocking(move || {
            db_clone.update_message(
                id,
                ProcessingStatus::Failed,
                Some(
                    "ERROR: Integrity check failed. Payload was tampered with before processing."
                        .into(),
                ),
            )
        })
        .await
        {
            Ok(Err(e)) => eprintln!("Failed to update tampered message {}: {}", id, e),
            Err(e) => eprintln!("Task panicked updating tampered message {}: {}", id, e),
            Ok(Ok(())) => {}
        }
        return;
    }

    // 1. Bypass Check
    if engine.should_bypass(&msg) {
        println!("Bypassing compaction for model: {:?}", msg.model);
        let processed = engine.process_bypass(msg);
        let status = processed.status;
        let processed_content = processed.processed_content;
        let db_clone = Arc::clone(&db);
        match tokio::task::spawn_blocking(move || {
            db_clone.update_message(id, status, processed_content)
        })
        .await
        {
            Ok(Err(e)) => eprintln!("Failed to update bypassed message {}: {}", id, e),
            Err(e) => eprintln!("Task panicked updating bypassed message {}: {}", id, e),
            Ok(Ok(())) => {}
        }
        return;
    }

    // Note: Message is already marked as 'processing' by poll_pending_messages in db.rs

    // 2. Sanctuary (Extract Code)
    let sanctuary = extract_code_blocks(&msg.raw_content);

    // 3. Distillation (Summarize)
    println!(
        "Summarizing text ({} blocks preserved)...",
        sanctuary.code_blocks.len()
    );
    let summary_result = summarizer.summarize(&sanctuary.sanitized_text).await;

    match summary_result {
        Ok(summary) => {
            // 4. Reassembly
            let final_content =
                restore_code_blocks(&summary, &sanctuary.code_blocks, &sanctuary.marker);
            let db_clone = Arc::clone(&db);
            match tokio::task::spawn_blocking(move || {
                db_clone.update_message(id, ProcessingStatus::Completed, Some(final_content))
            })
            .await
            {
                Ok(Ok(())) => println!("Message ID {} completed.", id),
                Ok(Err(e)) => eprintln!("Failed to mark message {} as completed: {}", id, e),
                Err(e) => eprintln!("Task panicked marking message {} as completed: {}", id, e),
            }
        }
        Err(e) => {
            eprintln!("Summarization failed for ID {}: {}", id, e);
            // Fail-open: Skip compaction but don't block the message
            let raw_content = msg.raw_content.clone();
            let db_clone = Arc::clone(&db);
            match tokio::task::spawn_blocking(move || {
                db_clone.update_message(id, ProcessingStatus::Skipped, Some(raw_content))
            })
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(e)) => eprintln!(
                    "Failed to mark message {} as skipped after error: {}",
                    id, e
                ),
                Err(e) => eprintln!(
                    "Task panicked marking message {} as skipped after error: {}",
                    id, e
                ),
            }
        }
    }
}
