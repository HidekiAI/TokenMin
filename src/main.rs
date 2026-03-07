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
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("TokenMin started.");

    let config = Config::from_env();
    let db = Db::new(&config.db_path)?;
    let engine = Engine::new(config.bypass_models.clone());

    // Summarizer::new now returns a Result
    let summarizer = Summarizer::new(config.ollama_url.clone(), config.ollama_model.clone())
        .map_err(|e| format!("Failed to initialize summarizer: {}", e))?;

    println!(
        "Polling for messages in: {} (Interval: {}ms)",
        config.db_path, config.poll_interval_ms
    );

    loop {
        match db.poll_pending_messages() {
            Ok(messages) => {
                for msg in messages {
                    process_message(&db, &engine, &summarizer, msg).await;
                }
            }
            Err(e) => eprintln!("Database error: {}", e),
        }
        sleep(Duration::from_millis(config.poll_interval_ms)).await;
    }
}

async fn process_message(db: &Db, engine: &Engine, summarizer: &Summarizer, msg: Message) {
    let id = msg.id;
    println!("Processing message ID: {}", id);

    // 1. Bypass Check
    if engine.should_bypass(&msg) {
        println!("Bypassing compaction for model: {:?}", msg.model);
        let processed = engine.process_bypass(msg);
        if let Err(e) = db.update_message(id, processed.status, processed.processed_content) {
            eprintln!("Failed to update bypassed message {}: {}", id, e);
        }
        return;
    }

    // Mark as processing
    if let Err(e) = db.update_message(id, ProcessingStatus::Processing, None) {
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
            if let Err(e) = db.update_message(id, ProcessingStatus::Completed, Some(final_content))
            {
                eprintln!("Failed to mark message {} as completed: {}", id, e);
            } else {
                println!("Message ID {} completed.", id);
            }
        }
        Err(e) => {
            eprintln!("Summarization failed for ID {}: {}", id, e);
            // Fail-open: Skip compaction but don't block the message
            if let Err(update_err) =
                db.update_message(id, ProcessingStatus::Skipped, Some(msg.raw_content))
            {
                eprintln!(
                    "Failed to mark message {} as skipped after error: {}",
                    id, update_err
                );
            }
        }
    }
}
