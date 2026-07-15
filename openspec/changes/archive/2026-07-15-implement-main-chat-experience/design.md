## Context

VaneHub AI currently has the surrounding product shell for a multi-agent desktop tool: agent registry and switching, session records, MCP management, theme/runtime controls, and a chat-first main layout. The missing piece is the actual conversation loop. The center workspace can show chat-shaped UI, but it does not yet provide a service-backed message list, streamed assistant response, stop action, or durable message history.

The change must preserve the existing runtime boundary:

```text
React components
      |
      v
AgentService interface
      |
      +-- WebAgentClient
      |     +-- mock messages and mock stream
      |
      +-- TauriAgentClient
            +-- invoke(send_message/list_messages/stop_generation)
            +-- subscribe to chat:event
                  |
                  v
             Rust Tauri layer
                  |
          +-------+--------+
          v                v
   SQLite messages    Agent runtime adapters
```

## Goals / Non-Goals

**Goals:**

- Render a complete message list in the main workspace for the active session.
- Send user messages through the frontend service boundary.
- Display streamed assistant content, thinking content, completion, failure, and cancellation states.
- Support a stop-generation action from the shared input button state.
- Persist desktop messages in SQLite and reload them by session.
- Keep Web runtime usable through mock messages and mock streaming.
- Add an Agent runtime/parser boundary so agent-specific stdout parsing does not leak into UI code.

**Non-Goals:**

- Prompt enhancement is not implemented; `EnhanceButton` remains a disabled placeholder.
- Rich prompt editing with `contentEditable`, slash commands, and file-reference chips is out of scope.
- Full protocol support for every Agent CLI is out of scope. The first real parser targets Claude Code, with a generic line fallback for others.
- Introducing React Query, Redux, Zustand, MobX, or another state library is out of scope.
- Direct React usage of Tauri `invoke()` or SQLite is out of scope.

## Decisions

### Message state is split between persistent messages and stream events

`ChatMessage` represents durable state that can be listed from a session. `ChatStreamEvent` represents incremental runtime events such as `started`, `token`, `thinking`, `completed`, `failed`, and `cancelled`.

Alternative considered: mutate a single message object shape for every stream chunk. That makes persistence, retries, and event ordering harder to reason about. Separate event types let the Web adapter, Tauri adapter, and UI reducer share one contract.

### Phase 2 validates UI through Web/mock streaming first

Phase 2 implements the service contract, message list, scroll behavior, send/stop state, and mock stream in `web-agent-client.ts` before the Rust pipe exists.

Alternative considered: build Rust process management first. That would couple UI iteration to CLI protocol uncertainty and delay front-end validation.

### React components subscribe through AgentService, not Tauri events

The frontend service contract exposes:

```ts
sendMessage(input): Promise<ChatMessage>
listMessages(input): Promise<ChatMessage[]>
stopGeneration(sessionId): Promise<void>
subscribeMessageEvents(sessionId, handler): Promise<() => void>
```

`tauri-agent-client.ts` maps this to `invoke()` and `listen("chat:event")`. `web-agent-client.ts` maps it to in-memory state and timers.

Alternative considered: call `listen("chat:token")` directly from components. That would break the Web adapter symmetry and violate the existing frontend/backend isolation rule.

### A single `chat:event` channel carries typed payloads

The Tauri backend emits one event name with a typed `ChatStreamEvent` payload. The payload `type` decides how the frontend applies the event.

Alternative considered: use many event names such as `chat:token`, `chat:complete`, and `chat:failed`. Multiple event names make lifecycle cleanup and Web parity more complex.

### SQLite messages belong to sessions

Desktop messages are stored in a `messages` table with `session_id`, `role`, `status`, content fields, token usage fields, metadata, and timestamps. The foreign key uses `ON DELETE CASCADE` so deleting a session deletes its messages.

Alternative considered: store transcript JSON on the session row. That makes pagination, partial updates during streaming, and status queries awkward.

### Agent runtime supports persistent and one-shot strategies

The runtime layer must not assume every Agent CLI supports a long-lived stdin session. Agents that support persistent stdin/stdout may reuse a process per session. Agents that do not may run one command per message and reconstruct context through adapter-specific arguments or future history passing.

Alternative considered: model every Agent as a persistent `HashMap<session_id, process>`. That is simple but too brittle for CLI tools with different interaction models.

### Stop generation prefers soft cancellation

`stop_generation` first attempts an agent-supported interrupt/cancel path. If that is unavailable, it cancels the active task; as a last resort it kills the process and marks the runtime as needing rebuild.

Alternative considered: always kill the child process. That loses persistent session context and makes stop behavior unnecessarily destructive.

## Risks / Trade-offs

- Agent CLI stdout formats may change or differ by version -> start with Claude Code parser, keep a generic line parser fallback, and persist unrecognized output as content or diagnostic metadata rather than dropping it.
- WebView event delivery may be bursty -> keep event payloads small and allow backend batching later without changing the service contract.
- Process lifecycle management can leak children -> track active generation state by session, add cleanup on completion/failure/cancel, and include Rust tests for parser/process-manager boundaries where practical.
- Message streaming can fight user scrolling -> auto-scroll only when the user is near the bottom and show a scroll-to-bottom control otherwise.
- Static model data can drift -> treat Phase 1 model data as UI seed data keyed by stable ids; future capability discovery can replace it behind the same config contract.
- Partial assistant content after cancellation can confuse status -> preserve generated content and mark the message `cancelled` instead of deleting it.

## Migration Plan

1. Add frontend message and stream-event types, service methods, and Web/mock implementation.
2. Replace static transcript placeholders with service-backed message list and stream reducer behavior.
3. Add SQLite migration and Rust message repository.
4. Add Tauri commands and frontend Tauri adapter methods.
5. Add Agent runtime manager and parser boundary, starting with Claude Code and generic fallback.
6. Validate with OpenSpec, frontend build/test coverage, and Rust checks.

Rollback is additive: UI can temporarily use the Web/mock implementation while desktop commands are incomplete. The SQLite migration is additive and does not remove existing session data.

## Open Questions

- Which Claude Code output mode should be the first parser target in implementation: JSON Lines, text stream, or a command-specific structured mode?
- Should message metadata include raw stderr excerpts by default, or only on failed generations?
- Should tool-use rendering initially show raw JSON, or should each known tool get a formatted view later?
