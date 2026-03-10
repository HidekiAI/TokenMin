# TokenMin: Future Architectural Plans

This document outlines the potential architectural paths for integrating the TokenMin context-compression engine with various CLI-based AI assistants (e.g., `gemini-cli`, `claude-cli`, `copilot-cli`) in a TTY/PTY environment.

## Architectural Options Matrix

| Feature / Approach | 1. Shared Memory SQLite Queue (Current Design) | 2. Local API Proxy (HTTP Server) | 3. MCP Server (Model Context Protocol via stdio) | 4. CLI Skills / Shell Wrappers (Direct Binary Exec) |
| :--- | :--- | :--- | :--- | :--- |
| **Core Mechanism** | CLIs write/read prompts to the shared queue DB configured via `$TOKENMIN_DB` (default `/dev/shm/chat_and_plan/message_queue.sqlite3`). TokenMin daemon watches and processes. | TokenMin acts as a local HTTP server (`localhost:8080`). CLIs point to it as their "API Base". | TokenMin exposes tools (e.g., `compress_context`) over standard I/O streams. | CLI pre-hooks or custom instructions would, in a future design, pipe text through a dedicated streaming helper (e.g., a hypothetical `tokenmin compress` subcommand or `tokenmin-compress` binary). |
| **Performance / Latency** | **Extremely High.** Sub-millisecond reads/writes via RAM disk (`/dev/shm`). Asynchronous. | **Medium.** Network stack overhead (even locally). Blocking HTTP requests. | **High.** Direct standard I/O streaming. Blocking during tool execution. | **High.** Process spawning overhead, but direct stream piping. |
| **CLI Compatibility** | **Low-Medium.** Requires CLI to support pre-request hooks or native `chat_and_plan` tool integrations. | **Very High.** Works with almost any CLI that allows overriding the `OPENAI_API_BASE` or provider URL. | **Medium-Growing.** Requires the CLI to natively support the MCP specification (e.g., Claude Desktop, some CLIs). | **Low-Medium.** Requires the CLI to support custom pre-execution bash scripts or robust prompt-based skills. |
| **Security (Local)** | **High (Request only).** Uses HMAC-SHA256 to verify incoming request integrity, but currently lacks a response signature (clients should treat DB response as untrusted). | **Medium.** HTTP endpoints can be hit by other local processes unless secured with a local auth token. | **High.** Controlled purely via parent-child process standard I/O. | **High.** Standard OS-level process permissions. |
| **Asynchronous Capability** | **Excellent.** TokenMin runs as a background daemon. Prompts queue and process independently. | **Poor.** The CLI agent blocks and waits for the HTTP response before continuing. | **Poor.** The CLI agent blocks while waiting for the tool to return the compressed string. | **Poor.** The CLI blocks while the shell command executes. |
| **Implementation Effort (for TokenMin)** | **Done.** This is the current architecture built in Rust. | **High.** Requires building an HTTP framework (e.g., Actix/Axum) and mimicking OpenAI/Anthropic API shapes. | **Medium.** Requires implementing the MCP JSON-RPC protocol over stdio in Rust. | **Medium.** Requires implementing a dedicated stdin/stdout execution mode in the Rust binary (including CLI argument parsing and a non-daemon, single-run control flow). |
| **Implementation Effort (for the CLIs)** | **High.** You must write custom adapter scripts or plugins for each CLI to talk to the SQLite DB. | **Low.** Usually just changing an environment variable (`export OPENAI_API_BASE=http://localhost:8080`). | **Low (if supported).** Just add the TokenMin executable path to the CLI's MCP config file. | **High.** Requires writing custom shell aliases or modifying the CLI's source/configuration. |
| **Best Used When...** | You want an asynchronous, ultra-low latency system and are willing to write custom hooks for your CLIs. | You want maximum compatibility with existing CLIs without modifying how they work internally. | You are using modern, MCP-compatible agents and want them to *choose* when to compress data. | You want a quick, "Unix philosophy" pipeline in a future streaming mode (e.g., `cat file | tokenmin-compress | claude-cli`), once such a helper exists. |

## Summary Recommendations

1. **If sticking to the current Rust architecture (`/dev/shm` SQLite):**
   Requires writing custom pre-execution hooks for `claude-cli` and `copilot-cli` so they know to drop their payloads into the database and wait for the HMAC-signed response before sending to the cloud. For `gemini-cli`, you can instead reuse its default `chat_and_plan` SQLite database (used for persisting conversations) as the shared queue path, rather than relying on any special tool integration.

2. **For the easiest integration with all CLIs today (The Universal Path):**
   The **Local API Proxy** is historically the path of least resistance. By pointing all CLIs to `localhost:8080`, TokenMin handles the compression invisibly. However, this requires adding an HTTP server layer (like Axum or Warp) to the Rust codebase.

3. **For the most "Agentic" approach (The Future Standard):**
   The **MCP Server** approach. Adapting TokenMin to speak the Model Context Protocol over `stdio` allows modern, MCP-compatible agents to natively understand they possess a "Compression Engine" tool and decide when to use it.
