## Context

`RuntimeAgentApiAdapter::execute()` (`src-tauri/src/contexts/agent_runtime/infrastructure/api_process_adapter.rs`) today does exactly one request/response turn: fetch credential → fetch `ApiProviderConfig` → build a request via a `WireFormat` selected by `interface_format` (`anthropic_provider.rs` / `openai_compatible_provider.rs`, both pure translation modules) → stream the SSE response, forwarding `Token`/`Thinking` events to the sink → return a single terminal `Completed`/`Failed`. There is no loop; nothing here has ever driven tool execution. Every CLI-based agent (Claude Code, Codex CLI, etc.) has its own tool-use loop and permission prompting inside the external binary VaneHub spawns — VaneHub's Rust backend only parses that binary's stdout into `ProviderOutputEvent`/`GenerationProcessEvent::ToolLifecycle`, it never executes a tool itself.

The frontend chat pipeline already renders `GenerationProcessEvent::ToolLifecycle(ToolLifecycleEvent)` (wrapping `ToolUseBlock { id, name, input, output, status }`, `ToolLifecyclePhase { Started, Updated, Completed, Failed }`) because CLI agents already produce it. `AgentMessage` already has a `tool_use: Vec<ToolUseBlock>` field for persisting which tools ran as part of a message. This change's job is to make the *native* agent produce the same vocabulary from tool calls it actually executes and gates itself, not to invent new frontend rendering.

Two existing `platform` primitives are confirmed fit for reuse (read in full this session, not just by name):
- `platform::process`: `ProcessRequest` (executable/args/current_dir/env/timeout/cancellation/output_limit builder) + `ProcessAdapter::execute()` → `ProcessOutput`, plus `audit_command` for logging. Args are passed as explicit `OsString` values, never concatenated into a shell string, so command injection isn't a risk in how the tool invokes it.
- `platform::filesystem`: `BoundedFilesystem::new(root)` + `validate_relative`/`resolve_existing`/`resolve_with_existing_parent`, enforcing that a relative path can never resolve (via traversal or symlink) outside `root`.

Per the user's explicit choice, the permission/approval model for this change is **risk-tiered by tool/operation, not per-call classification of content**: file reads execute immediately with no prompt; file writes and shell execution always require an explicit user approve/deny before running. This is a fixed property of which tool/operation is being called, not a judgment about the specific command or path.

## Goals / Non-Goals

**Goals:**
- Give the native agent (both `interface_format`s) an actual tool-use loop: request → tool call(s) → execute → tool result → request again → ... → final text-only response.
- Exactly two tools this phase: shell execution and file read/write, both sandboxed through the `platform::process`/`platform::filesystem` primitives above.
- A risk-tiered approval gate: reads auto-approved, writes/execution require an explicit user decision surfaced live in the chat UI, with no tool call executing before that decision (when required).
- Reuse the existing `ToolLifecycleEvent`/`ToolUseBlock` vocabulary and `AgentMessage.tool_use` persistence field; no new event vocabulary for basic tool lifecycle.
- Desktop-runtime real execution; Web/mock runtime gets deterministic simulated tool-call/approval/result sequences through the same service contract.

**Non-Goals:**
- Any tool beyond shell and file read/write (no web search, no additional file operations like move/delete as distinct tools — file write covers create/overwrite).
- Content-based risk classification (e.g. parsing a shell command to decide if *this particular* command is "safe") — the tier is fixed per tool/operation as stated above.
- Context compaction, a Skill system for API-based agents, cross-session memory, or an explicit separate "plan then execute" phase (this loop is tool-driven, not planning-driven) — all deferred to later phases per the ongoing multi-phase effort.
- Agent Terminal, CLI Parameter Management, and Prompt Hook integration — unaffected, still CLI-only.
- Resuming an in-progress tool-use loop across an app restart (see Decision 5) — out of scope this phase.

## Decisions

### 1. The loop lives inside `execute()` as a bounded iteration, not recursion or a new adapter

`execute()` gains an iteration loop (hard cap, e.g. 25 round trips per user message — exceeding it returns a non-retryable `Failed`) around the existing single-turn logic. Each iteration: send the current message list (original history + any tool turns accumulated so far this call) → stream the response → if the response contains no tool calls, treat its final text as the terminal `Completed` (today's behavior, unchanged) → if it contains one or more tool calls, execute them (Decision 3/4) and append the tool_use/tool_result turns to the in-memory message list for the next iteration.

**Why:** the existing thread-per-generation model (`monitor_generation` spawns one `std::thread`, cancellation is one shared `AtomicBool`, the sink streams events as they occur) already fits a longer-running loop with no structural change — a loop inside the same function reuses that thread, that cancellation flag, and that sink unchanged. A new adapter or recursive spawning would duplicate registration/cancellation bookkeeping for no benefit.

**Alternative considered:** recursion (`execute` calls itself after a tool result) — rejected; a bounded loop with an explicit counter is simpler to reason about and cap than recursion depth.

### 2. Tool-use turns are in-memory-only within one `execute()` call; only the final message is persisted

The tool_use/tool_result exchanges the provider needs to see for continuity *within* one user message's loop are kept as a local `Vec<Value>` (or equivalent) built from the initial `ConversationHistoryPort::recent_messages` fetch, appended to as the loop iterates, and never written back through `ConversationHistoryPort`/SQLite. Only the final response — with all `ToolUseBlock`s collected during the loop attached to its `AgentMessage.tool_use` — is persisted as today's single assistant message, via the existing sink/completion path.

**Why:** `AgentMessage.tool_use: Vec<ToolUseBlock>` and both wire-format `build_request_body` functions' `matches!(role, "user" | "assistant")` filter already exist and need no changes if intermediate tool turns never become their own persisted `AgentMessage` rows. Threading them as transient, provider-native JSON avoids inventing new message roles or a schema migration for something only needed for the duration of a single generation call.

**Trade-off, accepted:** if the app restarts mid-loop, the in-progress tool exchange is lost (same as an in-flight CLI generation today isn't resumable across a restart). Revisit only if a later phase needs durable mid-loop resumption.

### 3. Tool catalog: shell and file read/write, translated per wire format

Two tool definitions, defined once in a provider-agnostic shape (name, JSON Schema for input) and translated into each wire format's `tools` array shape (Anthropic's `{name, description, input_schema}`; OpenAI's `{type: "function", function: {name, description, parameters}}`). Tool-call detection and the reply-turn shape are also translated per format in `anthropic_provider.rs`/`openai_compatible_provider.rs`: Anthropic accumulates `tool_use` content blocks and a `tool_result` block goes in the next `user`-role message; OpenAI accumulates `tool_calls` off streamed deltas and replies via a `role: "tool"` message keyed by `tool_call_id`.

**Why:** both formats are already shipped (Phase 1 + the OpenAI-compatible scope revision); a tool-execution phase that only worked for one would be an inconsistent product surface. Keeping the two provider modules as the sole place wire-format differences live continues the pattern that made them independently unit-testable.

### 4. Risk-tiered approval, blocking the generation thread, resolved by a new Tauri command

Tool execution risk is fixed per operation: file **read** → auto-approved, executes immediately. File **write** and **shell execution** → always require approval. When a call needs approval, the adapter emits a new `ToolLifecyclePhase::AwaitingApproval` (added to the existing enum) carrying the `ToolUseBlock`, then blocks the generation's worker thread on a oneshot channel registered (keyed by `process_id` + `call_id`) in the same `Arc<Mutex<HashMap<...>>>` bookkeeping `RuntimeAgentApiAdapter` already uses for managed generations. A new Tauri command (e.g. `resolve_tool_approval(process_id, call_id, decision)`) looks up that entry and sends the decision, unblocking the thread; the thread then executes (approve) or emits `ToolLifecyclePhase::Failed` with a "denied by user" diagnostic (deny). The blocking wait also observes the existing cancellation `AtomicBool` (polled with a short timeout rather than an unbounded `recv()`), so `stop_generation` still works while a tool call is awaiting approval.

**Why:** this reuses the exact process/cancellation bookkeeping the adapter already has instead of inventing a parallel "pending operations" subsystem. Adding one new `ToolLifecyclePhase` variant is additive — existing CLI-agent stdout parsing never produces it (Claude Code's own approval flow is internal to that process and never reaches VaneHub's stdout parser), so this doesn't change CLI-agent behavior; the frontend gains one new case to render (an inline approve/deny prompt) alongside its existing phase handling.

**Alternative considered:** poll-based approval (frontend repeatedly asks "is there a pending approval?") — rejected in favor of blocking-thread-plus-push-event, which reuses the existing sink-based streaming the frontend already listens to instead of adding a new polling command.

### 5. Sandboxing: shell via bounded/timeout process execution, file access rooted at the session's workspace folder

The shell tool wraps `platform::process::ProcessRequest`/`ProcessAdapter` with a fixed timeout and output-size bound (both already parameters on `ProcessRequest`) and the existing `audit_command` logging hook. The file tool wraps `platform::filesystem::BoundedFilesystem::new(root)`, rooted at the session's `folder` (`AgentSession.folder: Option<String>` already exists) — a session with no folder cannot use the file tool (fails closed, non-retryable). Neither tool gets new sandboxing primitives invented from scratch.

**Why:** both primitives are already used and tested elsewhere in the codebase for exactly these properties (bounded/cancellable/timeout-limited subprocess execution; traversal/symlink-safe path resolution). Reimplementing either for this feature would duplicate already-hardened code.

### 6. Web/mock parity: deterministic simulated tool sequence, no real execution

`web-agent-client.ts`'s simulated generation for API-based agents gains a fixed, deterministic tool-call/approval/result sequence (e.g. one simulated file-read tool call that auto-completes, to exercise the rendering path) so the frontend contract and its tests don't silently diverge from desktop behavior. It does not attempt to simulate arbitrary shell/file operations.

**Why:** matches the existing Web/mock convention (`api-agent-runtime`'s existing "Web mock generation" requirement: deterministic simulated events, no real provider or OS access) rather than introducing a new exception for tool-related events.

## Risks / Trade-offs

- **[Risk]** A tool-use loop that never terminates (model keeps requesting tools) could run indefinitely and cost real API spend. → **Mitigation:** hard iteration cap (Decision 1); exceeding it fails the generation with a clear diagnostic rather than looping forever.
- **[Risk]** Blocking a thread on user approval could hang a generation indefinitely if the user never responds. → **Mitigation:** this mirrors how a live chat session already expects user attention; the block is still interruptible via the existing cancellation flag (Decision 4), so the user (or a future timeout) can always end it.
- **[Risk]** Shell execution is inherently powerful even when explicit-argument-safe (no shell-string injection) — a sandboxed *command* is not the same as a *safe* command. → **Mitigation:** this is exactly why shell execution is always gated behind approval regardless of the specific command (Decision 4/Non-Goals) rather than attempting content-based safety classification.
- **[Risk]** Adding `ToolLifecyclePhase::AwaitingApproval` touches a shared enum also produced by CLI-agent stdout parsing. → **Mitigation:** confirmed the CLI parser (`infrastructure/providers/output.rs`) only ever produces the four existing phases from patterns already in Claude Code/Codex/etc. stdout; a new variant is additive and unreachable from that path unless a future change explicitly wires it up.
- **[Trade-off]** No durable resumption of an in-progress tool loop across an app restart (Decision 2). Acceptable for this phase; revisit only if a later phase needs it.

## Migration Plan

Purely additive: one new `ToolLifecyclePhase` variant, a new Tauri command for approval resolution, no changes to existing message/session schema (tool turns are in-memory per Decision 2), no changes to CLI-agent behavior. Rollback is simply not calling a tool from the native agent — today's tool-free conversational loop keeps working unchanged for sessions where the model never requests a tool.

## Open Questions

- Exact JSON Schema for the two tools' input (shell: presumably `{command: string}`, maybe `cwd`; file: presumably `{operation: "read"|"write", path: string, content?: string}`) — finalize at task-execution time, constrained by Decision 3/5.
- Whether the shell tool needs a configurable timeout/output-bound per call or a single fixed constant is sufficient for this phase — default to a single fixed constant unless task-time investigation shows a clear need for per-call configuration.
