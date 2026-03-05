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
    let summarizer = Summarizer::new(config.ollama_url.clone(), config.ollama_model.clone());

    println!("Polling for messages in: {}", config.db_path);

    loop {
        match db.poll_pending_messages() {
            Ok(messages) => {
                for msg in messages {
                    process_message(&db, &engine, &summarizer, msg).await;
                }
            }
            Err(e) => eprintln!("Database error: {}", e),
        }
        sleep(Duration::from_millis(100)).await;
    }
}

async fn process_message(db: &Db, engine: &Engine, summarizer: &Summarizer, msg: Message) {
    let id = msg.id;
    println!("Processing message ID: {}", id);

    // 1. Bypass Check
    if engine.should_bypass(&msg) {
        println!("Bypassing compaction for model: {:?}", msg.model);
        let processed = engine.process_bypass(msg);
        let _ = db.update_message(id, processed.status, processed.processed_content);
        return;
    }

    // Mark as processing
    let _ = db.update_message(id, ProcessingStatus::Processing, None);

    // 2. Sanctuary (Extract Code)
    let sanctuary = extract_code_blocks(&msg.raw_content);

    // 3. Distillation (Summarize)
    println!("Summarizing text ({} blocks preserved)...", sanctuary.code_blocks.len());
    let summary_result = summarizer.summarize(&sanctuary.sanitized_text).await;

    match summary_result {
        Ok(summary) => {
            // 4. Reassembly
            let final_content = restore_code_blocks(&summary, &sanctuary.code_blocks);
            let _ = db.update_message(id, ProcessingStatus::Completed, Some(final_content));
            println!("Message ID {} completed.", id);
        }
        Err(e) => {
            eprintln!("Summarization failed for ID {}: {}", id, e);
            // Fail-open: Skip compaction but don't block the message
            let _ = db.update_message(id, ProcessingStatus::Skipped, Some(msg.raw_content));
        }
    }
}
