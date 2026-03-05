# TokenMin MEMORY.md (compact)

- Rust tool for compressing LLM prompts, preserving code blocks.
- Summarizes non-code text locally (Qwen 2.5 via Ollama).
- Automatically bypasses compaction for free models (e.g., Copilot GPT-4.1).
- Fast, private, uses shared memory (SQLite).
- Setup: install Ollama, Rust 2024, set TOKENMIN_DB, run `cargo run --release`.
- **MEMORY.md and INSTRUCTIONS.md must be updated together with README.md whenever approaches change, to keep documentation alive.**

## Project Overview
TokenMin is a Rust-based prompt pre-processor that reduces LLM API consumption by filtering and compressing chat context before sending it to remote providers. It is designed for developers to manage token costs while preserving technical data and code integrity.

## Key Features
- **Code Sanctuary (Regex-Based):** Code blocks are detected and preserved, bypassing compression to avoid corrupting code logic.
- **Context Distillation:** Non-code text is summarized using a local SLM (e.g., Qwen 2.5 via Ollama) to create dense, information-rich summaries.
- **Reassembly:** The original code is reinserted into the compressed summary, maintaining technical accuracy while reducing token count.
- **Compaction Bypass:** If the target model is free (e.g., Copilot GPT-4.1), compaction is automatically bypassed. No manual override or prompt injection is required.

- **Compaction Bypass:** If the target model is free (e.g., Copilot GPT-4.1), compaction is automatically bypassed. No manual override or prompt injection is required.

## Performance Goals
- Minimize token waste and financial cost.
- Achieve low latency using Rust concurrency and shared memory.
- Ensure privacy by performing all summarization locally before any cloud API call.

## Quick Start
- Requires Ollama and Rust 2024.
- Set SQLite path (optimized for shared memory):
  ```bash
  export TOKENMIN_DB="/dev/shm/chat_and_plan/message_queue.sqlite3"
  ```
- Run the watcher:
  ```bash
  cargo run --release
  ```

## Inspirations
- LLMLingua (Microsoft)
- FrugalGPT (Stanford)
- Selective-Context (SLM-based pruning)

---
This summary is auto-generated for Copilot/Gemini memory loading. Update if README changes.