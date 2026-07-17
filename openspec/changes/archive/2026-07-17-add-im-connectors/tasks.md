## 1. Dependency and Protocol Baseline

- [x] 1.1 Audit candidate Rust HTTP, WebSocket, keyring, QR, and tray dependencies for Windows, macOS, Linux, licenses, and packaged-binary compatibility
- [x] 1.2 Add the selected native dependencies and feature flags without adding a Node connector runtime or frontend platform SDKs
- [x] 1.3 Define recorded, redacted protocol fixtures and transport test doubles for Feishu, Telegram, DingTalk, WeCom, and personal WeChat
- [x] 1.4 Add or restore the repository `npm run lint` command and strict TypeScript/React lint configuration required by the project validation contract if it is still absent

## 2. Native Storage and Secure Credentials

- [x] 2.1 Add additive SQLite migrations for connector configs, routing settings, credential references, session bindings, inbound deduplication, and connector checkpoints
- [x] 2.2 Implement typed repositories for connector configuration, routing, bindings, deduplication retention, and checkpoints with migration and repository tests
- [x] 2.3 Implement the credential-store trait, OS keyring backend, stable connector account naming, and in-memory test backend
- [x] 2.4 Implement atomic connector configuration save, credential replacement, clearing, compensation on database failure, and tests proving secrets never enter SQLite
- [x] 2.5 Extend unified log redaction for connector secret field names, authorization data, external identities, message content, prompts, responses, headers, and protocol frames

## 3. Shared Session and Chat Runtime

- [x] 3.1 Move session creation into an internal service with explicit activating and non-activating creation modes while preserving existing Tauri command behavior
- [x] 3.2 Add IM session source metadata and database/model migration support without exposing external chat identifiers
- [x] 3.3 Extract message submission and CLI generation startup from the Tauri command into one internal chat service used by desktop and native callers
- [x] 3.4 Add exactly-once terminal completion signaling for completed, failed, and cancelled assistant messages
- [x] 3.5 Add regression tests for desktop chat events, persistence, runtime-session continuity, token accounting, errors, cancellation, and active-session stability after the refactor
- [x] 3.6 Remove stale IM bindings when sessions are deleted and test automatic binding recovery

## 4. Shared IM Runtime

- [x] 4.1 Define built-in connector descriptors, normalized inbound/outbound models, lifecycle/status types, transport traits, and connector adapter contracts
- [x] 4.2 Implement the runtime manager for asynchronous start, stop, restart, test, per-connector health, generation-safe reconfiguration, and graceful shutdown
- [x] 4.3 Implement inbound direct-text filtering, durable deduplication, fast acknowledgement handoff, and bounded dedup cleanup
- [x] 4.4 Implement binding resolution and non-activating session creation from validated default Agent and project settings
- [x] 4.5 Implement bounded per-chat FIFO serialization, concurrent execution across chats, and localized queue-full behavior
- [x] 4.6 Implement final-only completion dispatch, platform-aware ordered text splitting, failed-generation responses, and non-rerunning delivery-failure handling
- [x] 4.7 Apply the existing network proxy and bypass policy to new IM HTTP and WebSocket clients
- [x] 4.8 Add integration tests that run all five fake connectors concurrently through deduplication, binding, queueing, Agent completion, and outbound delivery

## 5. Feishu and Telegram Adapters

- [x] 5.1 Implement Feishu credential validation, tenant-token handling, bounded connection test, and redacted status mapping
- [x] 5.2 Implement Feishu WebSocket lifecycle, direct-text event normalization, acknowledgement, reconnect, and recorded-fixture tests
- [x] 5.3 Implement Feishu final text reply, length handling, error classification, and mocked API tests
- [x] 5.4 Implement Telegram Bot API credential normalization, `getMe` connection test, and webhook-conflict detection without deleting an existing webhook
- [x] 5.5 Implement Telegram `getUpdates` long polling, durable offsets, direct-text normalization, cancellation, reconnect, and recorded-fixture tests
- [x] 5.6 Implement Telegram final `sendMessage` delivery, ordered length splitting, error classification, and mocked API tests

## 6. DingTalk and WeCom Adapters

- [x] 6.1 Implement DingTalk credential validation, access-token handling, bounded connection test, and redacted status mapping
- [x] 6.2 Implement DingTalk Stream lifecycle, direct-text normalization, acknowledgement, reconnect, and recorded-fixture tests
- [x] 6.3 Implement DingTalk final text reply, length handling, error classification, and mocked API tests
- [x] 6.4 Implement WeCom intelligent-bot credential validation, bounded WebSocket authentication test, and redacted status mapping
- [x] 6.5 Implement WeCom intelligent-bot WebSocket lifecycle, direct-text normalization, acknowledgement, reconnect, and recorded-fixture tests
- [x] 6.6 Implement WeCom final text reply using the inbound request context where required, length handling, error classification, and mocked protocol tests

## 7. Personal WeChat Adapter

- [x] 7.1 Implement experimental personal WeChat descriptor, QR authorization creation, short-lived in-memory QR state, polling cancellation, and expiry tests
- [x] 7.2 Persist personal WeChat secret tokens and resumable session artifacts through the credential store without logging or storing QR payloads
- [x] 7.3 Implement iLink long polling, direct-text normalization, secret checkpoint restoration, context-token handling, and recorded-fixture tests
- [x] 7.4 Implement personal WeChat final text reply, authorization-expiry transition, disconnect/reauthorize flow, and mocked API tests
- [x] 7.5 Review current personal WeChat access and distribution terms and keep the connector experimental and disabled if the release gate is not satisfied

## 8. Native Commands and Frontend Service Boundary

- [x] 8.1 Add declared Rust commands for descriptors/status, routing, configuration, enable/disable/restart, bounded tests, authorization, clear, and binding reset
- [x] 8.2 Add TypeScript IM contracts and runtime validation/normalization without `any` or direct native types in components
- [x] 8.3 Implement `ImService`, the Tauri IM client, runtime adapter selection, and contract-conformance tests
- [x] 8.4 Implement the Web/mock IM client with deterministic states and actions and prove that submitted secret plaintext is not persisted
- [x] 8.5 Register connector startup after migration and make startup failures non-blocking and visible through safe health state and unified logging

## 9. IM Settings Experience

- [x] 9.1 Add the IM settings navigation definition, icon, search placeholder, page shell, and placement before Usage Statistics and About
- [x] 9.2 Implement the default Agent and project routing controls with service-backed project selection and field-level validation
- [x] 9.3 Implement stable expandable connector rows with statuses, enable toggles, masked credential forms, documentation, test, retry, clear, and connector-specific controls
- [x] 9.4 Implement the personal WeChat focused QR authorization surface with loading, expiry, cancellation, success, error, and reauthorization states
- [x] 9.5 Split the IM page into focused components so every React file stays below 300 lines and all components depend only on frontend services
- [x] 9.6 Add synchronized zh-CN and en translations for navigation, routing, platforms, statuses, setup, validation, actions, notices, errors, and accessible labels
- [x] 9.7 Add frontend tests for routing validation, credential placeholder safety, row-local pending states, Web/mock limitations, QR lifecycle, connector state transitions, i18n parity, and visible-text guardrails
- [x] 9.8 Verify responsive layout, focus states, contrast, clipping, and stable dimensions in both `futuristic` and `minimal` styles

## 10. Workspace Source Indicators

- [x] 10.1 Extend frontend session contracts and adapters with safe IM source metadata while preserving existing session data compatibility
- [x] 10.2 Add compact localized platform indicators to existing session cards using semantic tokens and stable layout dimensions
- [x] 10.3 Add tests proving IM sessions retain existing select, rename, pin, archive, restore, delete, and transcript behavior without revealing external identifiers

## 11. Tray and Background Lifecycle

- [x] 11.1 Add the Tauri tray icon and localized show, hide, and quit actions for supported desktop packages
- [x] 11.2 Intercept main-window close to hide only when tray initialization succeeds, and add the localized first-close background notice
- [x] 11.3 Implement explicit quit state, stop accepting connector work, bounded graceful connector shutdown, timeout logging, and process exit
- [x] 11.4 Add native tests for close interception decisions and lifecycle shutdown plus a documented packaged-app smoke test for tray restore and quit
- [x] 11.5 Verify browser/Web mode exposes no false tray or background-process capability

## 12. End-to-End Verification

- [x] 12.1 Add Playwright coverage for the IM settings page in zh-CN and en, both registered themes, and representative desktop and narrow widths
- [x] 12.2 Capture and inspect Playwright screenshots for both themes to confirm no blank panels, overlap, clipping, unreadable status tones, or credential leakage
- [x] 12.3 Add opt-in live smoke tests and a credential/environment checklist for one direct-text round trip on each platform without placing secrets in source or logs
- [x] 12.4 Run `npm run lint`, `npm run test`, and `npm run build` and fix all failures
- [x] 12.5 Run `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo clippy --manifest-path src-tauri/Cargo.toml` and fix all failures
- [x] 12.6 Run `npm run tauri -- info` and perform the packaged desktop tray and connector-startup smoke checks
- [x] 12.7 Run `openspec validate --specs --strict` and `openspec validate "add-im-connectors" --strict` and resolve every validation issue
