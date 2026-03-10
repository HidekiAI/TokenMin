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

## Alternatives

If you prefer to fork the `gemini-cli` repository or hardcode TokenMin as a core feature, there are two alternative integration points:

### 1. Direct Integration: Modifying `GeminiChat`
**Location:** `packages/core/src/core/geminiChat.ts` (inside `makeApiCallAndProcessStream`)

Rather than using the Hook System, you can hardcode TokenMin directly into the chat lifecycle. You would inject TokenMin directly before `this.client.generateContentStream` or `fireBeforeModelEvent` is called, explicitly waiting for the SQLite queue to return the compressed `requestContents` buffer before proceeding.

### 2. The Native Replacement: `ChatCompressionService`
**Location:** `packages/core/src/services/chatCompressionService.ts`

`gemini-cli` actually has a native compression service! By default, it uses a generic LLM summarization technique when the `DEFAULT_COMPRESSION_TOKEN_THRESHOLD` (50% of the model's token limit) is reached.

Rather than treating TokenMin as an external proxy or a pre-flight hook, you could replace or extend the `compress()` method in `ChatCompressionService`. When the CLI realizes the context is getting too large, it would invoke your TokenMin Rust daemon via SQLite instead of its default local TypeScript token-pruning logic.