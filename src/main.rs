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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("TokenMin started.");

    let config = Config::from_env();
    let db = Arc::new(Db::new(&config.db_path)?);
    let engine = Engine::new(config.bypass_models.clone());

    // Summarizer::new now returns a Result
    let summarizer = Summarizer::new(config.ollama_url.clone(), config.ollama_model.clone())
        .map_err(|e| format!("Failed to initialize summarizer: {}", e))?;

    println!(
        "Polling for messages in: {} (Interval: {}ms)",
        config.db_path, config.poll_interval_ms
    );

    loop {
        let db_clone = Arc::clone(&db);
        let poll_result = tokio::task::spawn_blocking(move || db_clone.poll_pending_messages()).await?;

        match poll_result {
            Ok(messages) => {
                for msg in messages {
                    process_message(Arc::clone(&db), &engine, &summarizer, msg).await;
                }
            }
            Err(e) => eprintln!("Database error: {}", e),
        }
        sleep(Duration::from_millis(config.poll_interval_ms)).await;
    }
}

async fn process_message(db: Arc<Db>, engine: &Engine, summarizer: &Summarizer, msg: Message) {
    let id = msg.id;
    println!("Processing message ID: {}", id);

    // 1. Bypass Check
    if engine.should_bypass(&msg) {
        println!("Bypassing compaction for model: {:?}", msg.model);
        let processed = engine.process_bypass(msg);
        let db_clone = Arc::clone(&db);
        let _ = tokio::task::spawn_blocking(move || {
            if let Err(e) = db_clone.update_message(id, processed.status, processed.processed_content) {
                eprintln!("Failed to update bypassed message {}: {}", id, e);
            }
        }).await;
        return;
    }

    // Mark as processing
    let db_clone = Arc::clone(&db);
    let update_result = tokio::task::spawn_blocking(move || {
        db_clone.update_message(id, ProcessingStatus::Processing, None)
    }).await.unwrap();

    if let Err(e) = update_result {
        eprintln!("Failed to mark message {} as processing: {}", id, e);
        return; // Don't proceed if we can't update status
    }

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
            let final_content = restore_code_blocks(&summary, &sanctuary.code_blocks);
            let db_clone = Arc::clone(&db);
            let update_result = tokio::task::spawn_blocking(move || {
                db_clone.update_message(id, ProcessingStatus::Completed, Some(final_content))
            }).await.unwrap();

            if let Err(e) = update_result {
                eprintln!("Failed to mark message {} as completed: {}", id, e);
            } else {
                println!("Message ID {} completed.", id);
            }
        }
        Err(e) => {
            eprintln!("Summarization failed for ID {}: {}", id, e);
            // Fail-open: Skip compaction but don't block the message
            let raw_content = msg.raw_content.clone();
            let db_clone = Arc::clone(&db);
            let update_result = tokio::task::spawn_blocking(move || {
                db_clone.update_message(id, ProcessingStatus::Skipped, Some(raw_content))
            }).await.unwrap();

            if let Err(update_err) = update_result {
                eprintln!(
                    "Failed to mark message {} as skipped after error: {}",
                    id, update_err
                );
            }
        }
    }
}
