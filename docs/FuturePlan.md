# TokenMin: Future Architectural Plans

This document outlines the potential architectural paths for integrating the TokenMin context-compression engine with various CLI-based AI assistants (e.g., `gemini-cli`, `claude-cli`, `copilot-cli`) in a TTY/PTY environment.

## 📊 High-Level Comparison Matrix

| Feature | 1. Shared SQLite Queue | 2. Local API Proxy | 3. MCP Server | 4. Shell Wrappers | 5. Native Plugins / Memory |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Latency / Performance** | Extremely High | Medium | High | High | Extremely High |
| **CLI Compatibility** | Low/Medium | Very High | Medium/Growing | Low/Medium | Low (Highly Coupled) |
| **Security (Local)** | High (Request only) | Medium | High | High | High |
| **Asynchronous UI** | Excellent | Poor (Blocking) | Poor (Blocking) | Poor (Blocking) | Excellent |
| **Effort (TokenMin)** | Done (Current) | High | Medium | Medium | High |
| **Effort (CLIs)** | High | Low | Low | High | High |

---

## 🔍 Detailed Breakdown

### 1. Shared Memory SQLite Queue (Current Design)
**Core Mechanism:** CLIs write/read prompts to the shared queue DB configured via `$TOKENMIN_DB` (default `/dev/shm/chat_and_plan/message_queue.sqlite3`). The TokenMin daemon watches and processes them asynchronously.

*   **Pros:**
    *   **Extremely High Performance:** Sub-millisecond reads/writes via RAM disk (`/dev/shm`).
    *   **Asynchronous:** TokenMin runs as a background daemon. Prompts queue and process independently.
    *   **Security:** Uses HMAC-SHA256 to verify incoming request integrity.
*   **Cons:**
    *   **Implementation Effort (CLIs):** High. You must write custom adapter scripts or plugins for each CLI to talk to the SQLite DB.
    *   **Trust:** Currently lacks a response signature (clients should treat DB response as untrusted).
*   **Best Used When:** You want an asynchronous, ultra-low latency system and are willing to write custom hooks for your CLIs.

### 2. Local API Proxy (HTTP Server)
**Core Mechanism:** TokenMin acts as a local HTTP server (e.g., `localhost:8080`). CLIs point to it as their "API Base", intercepting the payload before forwarding it to the upstream cloud provider.

*   **Pros:**
    *   **Universal Compatibility:** Works instantly with almost any CLI that allows overriding the `OPENAI_API_BASE` or provider URL.
    *   **Low CLI Effort:** Usually just changing a single environment variable.
*   **Cons:**
    *   **Blocking:** The CLI agent blocks and waits for the HTTP response before continuing.
    *   **Network Overhead:** Adds network stack overhead (even locally).
    *   **Implementation Effort (TokenMin):** High. Requires building an HTTP framework (e.g., Actix/Axum) and mimicking OpenAI/Anthropic API shapes.
*   **Best Used When:** You want maximum compatibility with existing CLIs without modifying how they work internally.

### 3. MCP Server (Model Context Protocol via stdio)
**Core Mechanism:** TokenMin exposes specific tools (e.g., `compress_context`) over standard I/O streams using the MCP JSON-RPC protocol.

*   **Pros:**
    *   **Native Tooling:** Controlled purely via parent-child process standard I/O, feeling incredibly native to modern agents.
    *   **Security:** Inherently secure as it relies on parent-child process permissions.
*   **Cons:**
    *   **Blocking:** The CLI agent blocks while waiting for the tool to return the compressed string.
    *   **Adoption:** Requires the CLI to natively support the MCP specification (e.g., Claude Desktop, some CLIs).
*   **Best Used When:** You are using modern, MCP-compatible agents and want them to *choose* when to compress data.

### 4. CLI Skills / Shell Wrappers (Direct Binary Exec)
**Core Mechanism:** CLI pre-hooks or custom instructions would, in a future design, pipe text through a dedicated streaming helper (e.g., a hypothetical `tokenmin compress` subcommand or `tokenmin-compress` binary).

*   **Pros:**
    *   **Unix Philosophy:** Quick, scriptable pipeline integration (e.g., `cat file | tokenmin-compress | claude-cli`).
    *   **Security:** Standard OS-level process permissions.
*   **Cons:**
    *   **Implementation Effort (TokenMin):** Requires implementing a dedicated stdin/stdout execution mode in the Rust binary (including CLI argument parsing and a non-daemon control flow).
    *   **Blocking:** Process spawning overhead, and the CLI blocks while the shell command executes.
*   **Best Used When:** You want a quick, pipeline-driven approach for one-off CLI tasks, once a streaming helper exists.

### 5. Native Plugins & Dynamic Memory Alteration
**Core Mechanism:** TokenMin acts as an intelligent background archivist that integrates directly into the agent's internal state machine or extension API. This includes:
1. **SDK State Interception:** Silently compressing local session files (e.g., Anthropic's JSONL session history or `MEMORY.md`) *before* the CLI reads them to resume a session.
2. **Agentic Memory APIs:** Acting as the storage backend for an agent's explicit memory tools (e.g., Claude's Beta Memory API), where the agent explicitly requests TokenMin to `create`, `view`, or `str_replace` long-term facts.

*   **Pros:**
    *   **Zero Latency Overhead:** For session state interception, compression happens entirely asynchronously in the background.
    *   **Semantic Power:** When combined with Memory APIs, TokenMin becomes a smart knowledge-retrieval engine rather than just a dumb prompt-pipe.
*   **Cons:**
    *   **Brittle/Coupled:** Depends heavily on specific, undocumented file structures of individual CLIs, specific SDK schemas, or beta APIs.
    *   **High Maintenance:** Requires complex file-locking, reverse-engineering session states, or maintaining plugins specific to each CLI tool.
*   **Best Used When:** You want a "magic" asynchronous integration (SDK Interception) or want TokenMin to manage long-term cross-session knowledge (Memory API integration), and are willing to maintain highly coupled logic.

---

## 🎯 Summary Recommendations

1. **If sticking to the current Rust architecture (`/dev/shm` SQLite):**
   Requires writing custom pre-execution hooks for `claude-cli` and `copilot-cli` so they know to drop their payloads into the database and wait for the HMAC-signed response before sending to the cloud. For `gemini-cli`, you can instead reuse its default `chat_and_plan` SQLite database (used for persisting conversations) as the shared queue path, rather than relying on any special tool integration.

2. **For the easiest integration with all CLIs today (The Universal Path):**
   The **Local API Proxy** is historically the path of least resistance. By pointing all CLIs to `localhost:8080`, TokenMin handles the compression invisibly. However, this requires adding an HTTP server layer (like Axum or Warp) to the Rust codebase.

3. **For the most "Agentic" approach (The Future Standard):**
   The **MCP Server** approach. Adapting TokenMin to speak the Model Context Protocol over `stdio` allows modern, MCP-compatible agents to natively understand they possess a "Compression Engine" tool and decide when to use it.

4. **For deep, "invisible" optimization (Native Plugin/Memory Interception):**
   The **Dynamic Memory Alteration** approach. TokenMin operates entirely in the background, pruning the CLI's session files or context buffers directly. While providing the best user experience (zero blocking), it requires a high degree of coupling with specific CLI internal mechanics or maintaining dedicated plugins.