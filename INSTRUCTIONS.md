# Copilot Instructions for TokenMin

- **MEMORY.md must be kept concise and compact, as it is loaded for every prompt and directly impacts token usage.**
- **Both MEMORY.md and INSTRUCTIONS.md must be continuously updated alongside README.md whenever approaches or architecture change, to keep documentation alive and accurate.**

## Build, Test, and Lint Commands
- **Build:**
  - `cargo run --release` (requires Rust 2024)
- **Environment:**
  - Set the SQLite path for shared memory optimization:
    ```bash
    export TOKENMIN_DB="/dev/shm/chat_and_plan/message_queue.sqlite3"
    ```
- **Testing:**
  - No explicit test or lint commands found in the repository structure. If tests are added, use standard Rust conventions (`cargo test` for all tests, `cargo test <testname>` for a single test).

## High-Level Architecture
- **Purpose:** TokenMin is a Rust-based prompt pre-processor that reduces LLM API token usage by filtering and compressing chat context before sending it to remote providers.
- **Pipeline:**
  1. **Code Sanctuary:** Code blocks (detected via regex) are preserved and bypass compression to avoid corrupting code logic.
  2. **Context Distillation:** Non-code text is summarized using a local SLM (e.g., Qwen 2.5 via Ollama) to create dense, information-rich summaries.
  3. **Reassembly:** The original code is reinserted into the compressed summary, maintaining technical accuracy while reducing token count.
  4. **Compaction Bypass:** If the target model is free (e.g., Copilot GPT-4.1), compaction is automatically bypassed. No manual override or prompt injection is required.
- **Performance:**
  - Uses Rust concurrency and `/dev/shm` shared memory for low-latency database polling.
  - All summarization and scrubbing is performed locally for privacy.

## Key Conventions
- **Code blocks are immutable:** Any code block detected in the prompt context is never compressed or altered.
- **Local SLM summarization:** Only non-code text is summarized, and this is always done locally before any cloud API call.
- **No hardcoded credentials:** All configuration (such as database paths) is set via environment variables.

## Reference Docs
- See `README.md` and `MEMORY.md` for further details on architecture and usage.

---
This file was generated to help Copilot and other AI assistants understand the unique build, architecture, and conventions of TokenMin. Update as the project evolves.
