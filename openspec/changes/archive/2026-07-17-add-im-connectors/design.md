## Context

VaneHub currently starts Agent CLI work from the `send_message` Tauri command and persists sessions and messages in SQLite. That command owns validation, CLI process startup, streaming persistence, frontend event emission, and completion handling in one path. An IM connector cannot safely call a Tauri command from native code, and an external chat must not change the session currently selected in the desktop UI.

Clowder AI demonstrates the relevant domain split: platform adapters normalize inbound messages, a router performs deduplication and thread binding, and a shared outbound hook delivers completed responses. Its concrete implementation depends on a long-running Node API and Redis, so VaneHub will reuse the domain boundaries and platform behavior rather than its runtime or storage implementation.

The Tauri process is the only always-running local host. React must remain behind a frontend service boundary, the browser build must remain usable through mock data, SQLite access must stay in Rust, and every durable diagnostic must use unified logging. Connector credentials require stronger treatment than the current general settings table.

## Goals / Non-Goals

**Goals:**

- Support Feishu, Telegram, DingTalk, WeCom intelligent bot, and personal WeChat connectors concurrently and allow each connector to be enabled independently.
- Receive text direct messages, bind each external chat to one dedicated VaneHub session, run the configured Agent in the configured project, and deliver the final response.
- Keep external sessions visible in the normal session list without switching the desktop user's active session.
- Run connectors while the main window is hidden to the system tray and stop them gracefully only on explicit application quit.
- Store secrets in the operating-system credential store and keep structured non-secret state in SQLite.
- Provide equivalent Tauri and Web/mock frontend service contracts, localized settings, and both registered visual styles.
- Make connector behavior testable without live platform credentials through transport and credential-store abstractions.

**Non-Goals:**

- Group chat, mentions, multi-user authorization, images, files, voice, rich cards, interactive callbacks, or progressive streaming edits.
- Multiple accounts for the same platform.
- Receiving messages while the VaneHub process is not running.
- Webhook hosting, public ingress, system-wide background services, or an HTTP backend for browser-only mode.
- External connector plugins or user-authored connector manifests.
- Full feature parity between personal WeChat and the four stable bot platforms; personal WeChat remains experimental.

## Decisions

### 1. Use a native connector domain with platform adapters

Add `src-tauri/src/im/` with contracts for normalized inbound messages, outbound delivery, connector lifecycle handles, health snapshots, credential access, and transport clients. A shared runtime manager owns five adapter instances and exposes start, stop, restart, test, and status operations.

```text
React IM page
    |
    v
ImService -> TauriImClient -> IM commands
          -> WebImClient   -> deterministic mock
                              
Rust ImRuntimeManager
    +-- ConnectorAdapter (Feishu / Telegram / DingTalk / WeCom / WeChat)
    +-- InboundRouter
    +-- BindingStore / DedupStore
    +-- ChatService
    `-- OutboundDispatcher
```

Platform metadata is returned through a typed Rust registry rather than duplicated in React. The first version uses built-in code-defined descriptors, not YAML plugin loading, because there is no external plugin requirement and a runtime manifest would add validation and packaging surface without changing the five fixed implementations.

Alternative considered: port the Clowder TypeScript connector host or launch it as a Node sidecar. Rejected because it would introduce another backend runtime, make packaged desktop behavior depend on Node availability, and duplicate lifecycle and logging ownership.

### 2. Keep protocol clients in Rust and isolate unstable details

The runtime uses asynchronous Rust HTTP/WebSocket clients and platform-specific protocol modules:

- Feishu: application WebSocket events plus tenant token and message APIs.
- Telegram: Bot API `getUpdates` long polling and `sendMessage`.
- DingTalk: Stream connection and robot reply APIs.
- WeCom: intelligent-bot WebSocket API using Bot ID and Bot Secret.
- Personal WeChat: iLink Bot QR authorization, session state, long polling, and reply APIs.

Maintained Rust crates may be used when they cover the required protocol and can be packaged on all supported targets. Otherwise, the minimum protocol client is implemented behind a private transport trait from official protocol behavior and the Clowder reference. No platform wire types escape its adapter.

Personal WeChat is disabled until QR authorization succeeds, reports explicit authorization-expired state, and is labeled experimental. Its release remains gated on a live smoke test and a review of the platform's current access terms.

### 3. Refactor chat execution into a reusable internal service

Extract the native work currently owned by `send_message` into a `ChatService` entry point used by both the Tauri command and `InboundRouter`. The service accepts a session id, content, origin metadata, and an optional completion receiver. It continues to persist messages and emit existing `chat:event` events.

IM submission waits on a native completion signal rather than listening through React or polling the database. Completion delivers only the final assistant content. If a platform length limit requires multiple transport messages, splitting occurs after Agent completion and still counts as final-only delivery.

Session creation gains an internal `activate` option. Desktop-created sessions activate as today; IM-created sessions are persisted without changing `workflow_state.active_session_id`.

Alternative considered: duplicate CLI process execution inside the connector runtime. Rejected because it would fork persistence, cancellation, parsing, logging, and token-accounting behavior.

### 4. Bind one external direct chat to one dedicated session

`im_session_bindings` uses `(connector_id, external_chat_id)` as a unique key and stores the VaneHub session id. On the first accepted message, the router snapshots the configured default Agent and project into a new CLI session. Later messages reuse that session, preserving runtime session continuity.

Changing global defaults affects only new bindings. Existing bindings remain stable until explicitly reset. The external chat id remains in the binding table and is not exposed in general frontend session models; the session stores only `source_type = im` and `source_connector_id` for UI identification.

The first version requires a valid default Agent id and project path before any connector can be enabled. Agent selection uses stable registry ids and checks availability separately from launching a message.

### 5. Serialize work per external chat and deduplicate before execution

The router records platform event ids in `im_inbound_dedup` before scheduling Agent execution. Duplicate events are acknowledged but do not create messages or CLI processes. Dedup records expire through bounded maintenance.

Each binding has a bounded FIFO queue and at most one active generation. Different external chats may run concurrently. When a queue is full, the connector returns a localized busy response without silently dropping an accepted message. Long-lived platform callbacks acknowledge receipt quickly and run Agent work outside the platform callback deadline.

### 6. Persist structured state in SQLite and secrets in the OS credential store

SQLite migrations add:

- `im_connector_configs`: enabled state and non-secret field JSON.
- `im_routing_settings`: singleton default Agent and project.
- `im_credential_refs`: credential presence/reference metadata, never secret values.
- `im_session_bindings`: external chat to VaneHub session mapping.
- `im_inbound_dedup`: processed platform event ids and timestamps.
- `im_connector_checkpoints`: polling offsets and non-secret resumable state.

A credential-store trait is backed by the platform credential manager through a maintained Rust keyring library and by an in-memory fake in tests. Sensitive updates write credentials first, commit references/config in a database transaction, and compensate credential writes if the transaction fails. Clearing a connector removes its credential entries and stops its runtime.

Personal WeChat authorization tokens and other secret session artifacts also use the credential store. QR images are held in memory with a short expiry and are never written to logs or SQLite.

Alternative considered: store secrets in the existing SQLite settings table. Rejected because it is plain application data and cannot provide an OS-protected secret boundary.

### 7. Use explicit lifecycle and health states

Connector states are `unconfigured`, `disabled`, `connecting`, `connected`, `reconnecting`, `authorization-expired`, and `error`. Status includes timestamps and a concise redacted error, but never secret values or raw message content.

Enabled connectors start asynchronously after Tauri setup and database migration. Configuration changes restart only the affected connector. Transient failures use bounded exponential backoff with jitter; authentication failures stop automatic retry until credentials change or the user retries.

Network clients use the existing VaneHub-managed proxy configuration. A connector test performs bounded authentication/connectivity checks without starting a persistent inbound loop or sending a user-visible chat message.

### 8. Treat close-to-tray as process lifecycle, not connector behavior

Tauri owns a tray icon with show/hide and quit actions. A main-window close request is prevented and hides the window. An explicit quit flag bypasses close interception, asks the IM runtime manager to stop all connectors with a bounded timeout, and then exits.

The tray behavior is desktop-only. Browser/mock mode does not emulate native window lifecycle. If tray initialization fails, VaneHub keeps normal visible-window behavior and records a redacted native warning rather than trapping the user in an uncloseable process.

### 9. Add a dedicated frontend IM service and settings page

`ImService` provides connector descriptors, statuses, routing settings, configuration mutation, enable/disable, test, restart, QR authorization actions, and binding reset. `TauriImClient` is the only frontend module that invokes IM commands. `WebImClient` supplies deterministic connector states and simulated operations without persisting submitted secrets.

The IM page uses existing settings primitives and semantic tokens. It contains a compact routing section and one expandable row per platform, with stable dimensions, status, enable toggle, masked credentials, connection test, documentation action, and platform-specific authorization controls. All visible copy and accessible labels use synchronized zh-CN and en resources. The page does not branch on `futuristic` versus `minimal`; theme tokens provide the differences.

### 10. Extend unified logging with connector-safe context

Connector logs use the unified logging service with connector id, lifecycle operation, status, retry count, and opaque internal ids where useful. Redaction occurs before persistence and covers configured credential field names, bearer values, QR payloads, authorization tokens, external chat/user ids, message bodies, prompts, and Agent responses.

The settings page receives concise service errors. It never displays raw protocol frames, SDK output, or unredacted native diagnostics.

## Risks / Trade-offs

- [Five platform protocols create a large first-version surface] -> Share routing, persistence, lifecycle, and tests; keep adapter contracts narrow; implement and verify adapters incrementally behind independent enablement.
- [Feishu, DingTalk, and WeCom do not provide first-party Rust SDK coverage equivalent to Node] -> Isolate wire protocols, prefer maintained crates where viable, add recorded-fixture contract tests, and require a live smoke gate for each connector.
- [Personal WeChat access or token behavior may change] -> Label it experimental, disable it until explicit QR authorization, isolate its state, and allow it to be disabled without affecting the other connectors.
- [OS credential-store availability varies across desktop environments] -> Surface a blocked configuration state with remediation; never silently fall back to plaintext storage.
- [Closing to tray can surprise users] -> Show a one-time localized notice, provide an obvious tray quit action, and preserve an explicit in-app quit path.
- [Agent jobs can outlive platform callback deadlines] -> Acknowledge inbound events before execution, persist dedup state first, and deliver the final result asynchronously.
- [Long replies exceed platform limits] -> Split only after completion using platform-aware limits and preserve ordering.
- [Configuration changes during active work can race with delivery] -> Snapshot the adapter generation and binding for each queued job; drain or fail with a concise retry message before replacing the adapter.

## Migration Plan

1. Add additive SQLite migrations and new Rust/frontend contracts without enabling connectors.
2. Refactor chat submission and session creation behind internal services while preserving all existing command behavior and tests.
3. Add credential-store and connector runtime abstractions with fake transports.
4. Implement the five adapters and enable them independently after their contract and live smoke gates pass.
5. Add the settings page, Web/mock adapter, session source indicator, translations, and tray lifecycle.
6. On startup after upgrade, all connectors are disabled and unconfigured; no migration reads environment variables or existing general settings as secrets.
7. Rollback leaves additive tables and credential entries unused. A downgrade does not delete bindings or secrets automatically; users can clear credentials before downgrade.

## Open Questions

- Confirm the currently permitted distribution and authorization terms for the personal WeChat iLink Bot flow before marking that connector release-ready; failure leaves only that connector experimental/disabled and does not block the other four.
- Select concrete Rust protocol/keyring crate versions during implementation after confirming Windows, macOS, and Linux packaging compatibility and license requirements.
