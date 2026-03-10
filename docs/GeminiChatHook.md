# Gemini CLI Integration: The `BeforeModel` Hook

Based on an evaluation of the `gemini-cli` source code, TokenMin can be integrated natively into the chat lifecycle without modifying the core network proxy code. The CLI features a highly modular architecture with established interception points that perfectly map to TokenMin's design.

## The Ideal Path: The `BeforeModel` Hook

**Location:** `packages/core/src/core/geminiChat.ts` (inside `makeApiCallAndProcessStream`)

The `gemini-cli` features a robust Hook System (`packages/core/src/hooks/hookSystem.ts`). Right before the CLI makes an API call to the LLM, it fires a `BeforeModel` event.

### How it works natively:
```typescript
const beforeModelResult = await hookSystem.fireBeforeModelEvent({
  model: modelToUse,
  config,
  contents: contentsToUse,
});

if (beforeModelResult.modifiedContents) {
  contentsToUse = beforeModelResult.modifiedContents as Content[];
}
```

### Integration Strategy (Option 1 & Option 5)
This hook is the exact choke-point needed for **Shared Memory SQLite (Option 1)** or **Native Plugins (Option 5)** from our `FuturePlan.md`. 

You can write a Command Hook or a native CLI Extension that listens for the `BeforeModel` event:
1. The CLI hands the hook the raw `contentsToUse` (the bloated chat history).
2. Your hook intercepts this array and drops the text payload into TokenMin's `/dev/shm` SQLite DB.
3. TokenMin compresses it asynchronously.
4. The hook reads the compacted result back and returns it to the CLI as `modifiedContents`.
5. The CLI automatically uses your tiny, compressed prompt to make the remote HTTP request.

**Advantage:** This requires zero forks of the `gemini-cli` repository. It utilizes the official extension API to invisibly optimize the prompt before it ever costs tokens.

---

## 🔐 Security & Optimization Considerations

### Prompt Injection via `BeforeModel` Hook
Because the `BeforeModel` hook has the power to arbitrarily rewrite the `contentsToUse` array, it introduces a potential vector for prompt injection or context tampering if a malicious local process gains control of the hook execution or the SQLite queue.
* **The Mitigation (SQLite Mode):** This is precisely why TokenMin enforces **HMAC-SHA256 signatures** when operating in its default SQLite daemon mode. The `gemini-cli` hook must sign the payload before dropping it into `/dev/shm`, ensuring that no other local user or rogue background process can successfully inject fabricated context into the queue before TokenMin processes it.

### Future Architecture: FFI / Direct Memory Piping (No SQLite)
While the `/dev/shm` SQLite queue provides excellent asynchronous decoupling across different languages (TypeScript CLI -> Rust Daemon), the overhead of serializing, signing (HMAC), writing to SQLite, reading, and deserializing may be unnecessary if TokenMin is loaded directly into the CLI's memory space.

As a future optimization specifically for native integrations like the `gemini-cli` hook, TokenMin could expose a **Direct Memory / FFI (Foreign Function Interface) Mode**.
* **How it works:** TokenMin would be compiled as a native Node.js addon (via N-API / Neon) or a WebAssembly (Wasm) module.
* **The Benefit:** The `BeforeModel` hook would simply call `TokenMin.compress(contentsToUse)` synchronously in-memory. 
* **The Security Impact:** This entirely eliminates the need for the SQLite database, shared memory bounds, and HMAC signing. Because the data never leaves the Node.js process memory space, local filesystem tampering vectors are inherently mitigated.

---

## 🚫 Rejected Alternatives

During the architectural evaluation, we considered two other integration points within the `gemini-cli` source code. Both were rejected because they would require maintaining a hard fork of the `gemini-cli` repository, which violates our goal of a native, seamless integration.

### 1. Direct Integration: Modifying `GeminiChat`
**Location:** `packages/core/src/core/geminiChat.ts` (inside `makeApiCallAndProcessStream`)

Instead of using the Hook System, this approach involved hardcoding TokenMin directly into the chat lifecycle. We would inject TokenMin directly before `this.client.generateContentStream` or `fireBeforeModelEvent` is called, explicitly waiting for the SQLite queue to return the compressed `requestContents` buffer before proceeding. 

**Why it was rejected:** Requires maintaining a permanent fork of `gemini-cli` just to inject the SQLite polling logic.

### 2. The Native Replacement: `ChatCompressionService`
**Location:** `packages/core/src/services/chatCompressionService.ts`

`gemini-cli` actually has a native compression service! By default, it uses a generic LLM summarization technique when the `DEFAULT_COMPRESSION_TOKEN_THRESHOLD` (50% of the model's token limit) is reached.

This approach involved replacing or extending the `compress()` method in `ChatCompressionService`. When the CLI realized the context was getting too large, it would invoke the TokenMin Rust daemon via SQLite instead of its default local TypeScript token-pruning logic.

**Why it was rejected:** While semantically the "correct" place for compression logic, extending this internal class currently requires modifying the source code and maintaining a fork, whereas the `BeforeModel` hook provides the same prompt-interception capability via officially supported extensions.