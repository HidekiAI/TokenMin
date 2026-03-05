# TokenMin Technical Design Document

## 1. System Architecture

TokenMin operates as a local "sidecar" service that intercepts and optimizes chat messages before they are sent to a remote LLM provider.

### Components
1.  **Client Application**: The user interface (CLI, Web, IDE plugin) that generates chat messages.
2.  **Shared State (SQLite)**: A lightweight, file-based queue located in `/dev/shm` (Linux shared memory) for sub-millisecond latency. Acts as the IPC mechanism.
3.  **TokenMin Watcher (Rust)**: A background daemon that polls the database for `pending` messages, processes them, and updates their state.
4.  **Local SLM (Ollama)**: A locally running Small Language Model (e.g., Qwen 2.5-Coder) used for summarization.

### Data Flow
1.  **Write**: Client inserts a new message into the SQLite `messages` table with status `pending`.
2.  **Detect**: TokenMin Watcher detects the `pending` message via polling (or notification).
3.  **Process**:
    *   **Sanctuary**: Extract code blocks to preserve them.
    *   **Distill**: Summarize the remaining text using the Local SLM.
    *   **Reassemble**: Combine the summary and preserved code blocks.
4.  **Update**: TokenMin updates the message in SQLite with `processed_content` and status `completed`.
5.  **Read**: Client reads the `processed_content` and sends it to the remote LLM.

---

## 2. Database Schema

**File Location:** `$TOKENMIN_DB` (Default: `/dev/shm/tokenmin/queue.db`)

### Table: `messages`

| Column | Type | Description |
| :--- | :--- | :--- |
| `id` | `INTEGER PRIMARY KEY AUTOINCREMENT` | Unique message ID. |
| `session_id` | `TEXT NOT NULL` | Grouping for chat threads. |
| `role` | `TEXT NOT NULL` | `user`, `system`, or `assistant`. |
| `raw_content` | `TEXT NOT NULL` | The original input from the user. |
| `processed_content` | `TEXT` | The optimized content (NULL until processed). |
| `status` | `TEXT NOT NULL` | `pending`, `processing`, `completed`, `failed`, `skipped`. |
| `model` | `TEXT` | Target model ID (e.g., `gpt-4`, `claude-3`). Used for bypass logic. |
| `created_at` | `INTEGER NOT NULL` | Unix timestamp (ms). |
| `updated_at` | `INTEGER` | Unix timestamp (ms). |

---

## 3. Rust Module Structure

The project will be structured as a binary crate with modular components.

```
src/
├── main.rs           # Entry point, setup, main polling loop.
├── config.rs         # Env var loading (TOKENMIN_DB, OLLAMA_URL, BYPASS_MODELS).
├── db.rs             # SQLite interactions (rusqlite).
├── engine/
│   ├── mod.rs        # Orchestrates the processing pipeline.
│   ├── sanctuary.rs  # Regex logic for code block extraction/insertion.
│   └── summarizer.rs # HTTP client for Ollama API (reqwest).
└── models.rs         # Shared structs (Message, ProcessingStatus).
```

### Key Structs

**`models.rs`**
```rust
#[derive(Debug, Clone)]
pub struct Message {
    pub id: i64,
    pub session_id: String,
    pub role: String,
    pub raw_content: String,
    pub processed_content: Option<String>,
    pub status: ProcessingStatus,
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessingStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Skipped, // Used for bypass
}
```

---

## 4. detailed Algorithms

### 4.1. Model Classification & Bypass (The "Free Tier" Check)

**Goal:** Skip expensive compression for free or unlimited models (e.g., `gpt-4-turbo` might be paid, but `copilot-chat` might be free).

**Logic:**
1.  **Load Config:** Read `BYPASS_MODELS` env var (comma-separated list, e.g., "copilot-chat,gpt-3.5-turbo").
2.  **Check:** When a message is picked up:
    *   If `message.model` is present AND matches an entry in `BYPASS_MODELS`.
    *   **Action:**
        *   Update `status` to `Skipped`.
        *   Copy `raw_content` directly to `processed_content`.
        *   **Return early** (Skip Sanctuary, Distillation, Reassembly).
3.  **Otherwise:** Proceed to Step 4.2 (Sanctuary).

### 4.2. The Sanctuary (Code Preservation)

**Goal:** Prevent the LLM from mutating code.

**Logic:**
1.  Scan `raw_content` using Regex: `r"```(\w+)?\n([\s\S]*?)```"`.
2.  Replace each match with a placeholder token: `<<CODE_BLOCK_0>>`, `<<CODE_BLOCK_1>>`.
3.  Store the extracted code blocks in a `Vec<String>`.
4.  Return `(sanitized_text, code_blocks)`.

### 4.3. Context Distillation (Summarization)

**Goal:** Compress non-code text.

**Logic:**
1.  Receive `sanitized_text`.
2.  Construct a prompt for Ollama:
    > "Summarize the following text concisely, retaining all key technical constraints and request details. Do not output conversational filler. Text: {sanitized_text}"
3.  Send to Ollama API (`/api/generate`).
4.  Receive `summary_text`.

### 4.4. Reassembly

**Goal:** Restore the message.

**Logic:**
1.  Take `summary_text`.
2.  Iterate through `code_blocks`.
3.  If the placeholder `<<CODE_BLOCK_N>>` exists in the summary, replace it with the original code.
4.  **Critical Safety Fallback:** If the LLM hallucinated and removed a placeholder, append the "orphaned" code block to the end of the message to ensure no code is lost.

---

## 5. Test-Driven Development (TDD) Plan

We will build the system strictly following TDD.

### Phase 1: Core Logic (The Engine)
*   **Test 1.1: Bypass Logic**
    *   Input: Message with model="copilot-chat", Config bypass="copilot-chat".
    *   Assert: Status becomes `Skipped`, content is copied.
*   **Test 1.2: Sanctuary Extraction**
    *   Input: Text with mixed code blocks (Rust, Python) and prose.
    *   Assert: Code blocks are extracted exactly. Placeholders are inserted.
*   **Test 1.3: Sanctuary Reassembly**
    *   Input: Summary text with placeholders + List of code blocks.
    *   Assert: Final string contains original code.
*   **Test 1.4: Reassembly Safety (Orphaned Blocks)**
    *   Input: Summary text *missing* a placeholder.
    *   Assert: Missing code block is appended to the end.

### Phase 2: Database Layer
*   **Test 2.1: Schema Creation**
    *   Action: Initialize DB.
    *   Assert: Tables exist.
*   **Test 2.2: Queue Operations**
    *   Action: Insert `pending` message.
    *   Assert: `poll_pending_messages()` returns it.
    *   Action: Update status to `completed`.
    *   Assert: Message is updated.

### Phase 3: External Integration (Mocked)
*   **Test 3.1: Config Loading**
    *   Action: Set env vars.
    *   Assert: Config struct reflects values.
*   **Test 3.2: Summarizer Client**
    *   Use `mockito` to mock Ollama endpoint.
    *   Assert: Request body is correct; Response is parsed.

### Phase 4: Integration
*   **Test 4.1: Full Pipeline**
    *   Simulate full flow: DB Insert -> Watcher Detect -> Mock Summarize -> DB Update.

---

## 6. Error Handling Strategy

1.  **Database Locking:** Use `wal` mode (Write-Ahead Logging) in SQLite for better concurrency. Handle `SQLITE_BUSY` with a short retry loop.
2.  **Ollama Failure:** If the local LLM is down or times out:
    *   Log the error.
    *   Mark status as `Skipped` (Fail-Open).
    *   Copy `raw_content` to `processed_content`. **Never block the user because optimization failed.**

## 7. Next Steps
1.  Initialize `cargo init`.
2.  Add dependencies: `rusqlite`, `reqwest`, `tokio`, `serde`, `regex`, `dotenv`.
3.  Begin Phase 1 of TDD.
