# TokenMin README Summary for Copilot/Gemini

## Project Overview
TokenMin is a Rust-based prompt pre-processor that reduces LLM API consumption by filtering and compressing chat context before sending it to remote providers. It is designed for developers to manage token costs while preserving technical data and code integrity.

## Key Features
- **Code Sanctuary (Regex-Based):** Code blocks are detected and preserved, bypassing compression to avoid corrupting code logic.
- **Context Distillation:** Non-code text is summarized using a local SLM (e.g., Qwen 2.5 via Ollama) to create dense, information-rich summaries.
- **Reassembly:** The original code is reinserted into the compressed summary, maintaining technical accuracy while reducing token count.

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