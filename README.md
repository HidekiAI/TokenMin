# 🪙 TokenMin

**TokenMin** is a Rust-based prompt pre-processor designed to reduce LLM API consumption by filtering and compressing chat context before it is sent to remote providers. It is designed for developers who want to manage token costs without compromising the integrity of technical data or code.

## 🔬 Inspiration & Acknowledgments
TokenMin is built upon the principles and research of several key projects in the "frugal AI" space:

* **[LLMLingua (Microsoft)](https://github.com/microsoft/LLMLingua):** A coarse-to-fine prompt compression method that identifies and removes non-essential tokens.
* **[FrugalGPT (Stanford)](https://arxiv.org/abs/2305.05176):** A framework for reducing LLM costs through model cascading and prompt adaptation.
* **Selective-Context:** The concept of using small language models (SLMs) to calculate self-information and prune less informative content.

## 🛠️ The Architecture
TokenMin acts as a "trash compactor" for your LLM context. Instead of sending raw, redundant chat logs to expensive models (like Gemini Pro), it performs a three-step local pipeline:

1.  **Code Sanctuary (Regex-Based):** Identifies code blocks (```). These segments are treated as "immutable" and are bypassed by the compression engine to prevent the corruption of indentation-centric logic (Python, F#, etc.).
2.  **Context Distillation:** The non-code "text" segments are sent to a local SLM (e.g., Qwen 2.5 via Ollama). The model is tasked with generating a dense, information-heavy summary of the conversation history.
3.  **Reassembly:** The original code is re-inserted into the compressed summary, resulting in a smaller payload that aims to retain the original technical logic while reducing token overhead.

### Compaction Bypass
- If the target model is known to be free (e.g., Copilot GPT-4.1), compaction is automatically bypassed.
- No prompt injection or manual override is required; detection is automatic.
- Future support for slash commands or plugin flags may be added for explicit bypass or formatting control.


### Compaction Bypass
- If the target model is known to be free (e.g., Copilot GPT-4.1), compaction is automatically bypassed.
- No prompt injection or manual override is required; detection is automatic.
- Future support for slash commands or plugin flags may be added for explicit bypass or formatting control.


## 📊 Performance Goals
While efficiency varies based on the nature of the input, TokenMin aims to:

* **Minimize Financial Waste:** Reduce the input token count of conversational "filler."
* **Low Latency:** Utilize Rust’s concurrency and `/dev/shm` (shared memory) for sub-millisecond database polling.
* **Privacy First:** Perform all summarization and "scrubbing" locally before any data reaches a cloud API.

## 🚀 Quick Start (Development)
TokenMin is currently a work-in-progress. It requires **Ollama** and a **Rust 2024** environment.

```bash
# Set up all dependencies (Ollama, Rust, etc.)
bash ./scripts/setup.sh [local|lxd|docker]

# Set your SQLite path (optimized for shared memory)
export TOKENMIN_DB="/dev/shm/chat_and_plan/message_queue.sqlite3"

# Configure models to bypass compaction (comma-separated)
export BYPASS_MODELS="copilot-chat,gpt-3.5-turbo,claude-instant-1"

# Run the watcher
cargo run --release
```

- `setup.sh` will call all required setup scripts (Ollama, Rust, etc.) to get the workspace ready for development and testing.
- **MEMORY.md and INSTRUCTIONS.md must be updated together with README.md whenever approaches change, to keep documentation alive.**

