## 1. Correctness fixes

- [x] 1.1 Add Rust regression tests proving an existing IM binding continues through its persisted Agent, project, chat configuration, and provider continuity after global routing defaults change.
- [x] 1.2 Refactor inbound routing so global defaults are used only by new or stale binding creation and existing bindings execute exclusively from their persisted session configuration.
- [x] 1.3 Add typed connector field metadata and tests that classify public and secret fields for Feishu, Telegram, DingTalk, WeCom, and personal WeChat.
- [x] 1.4 Add failing tests for partial connector edits, omitted stored fields, invalid merged candidates, unavailable credential storage, and legacy all-in-one credential payloads.
- [x] 1.5 Implement native credential patch merging, complete-candidate validation before runtime mutation, public-field persistence, and idempotent legacy credential splitting without plaintext fallback.
- [x] 1.6 Align the Tauri and Web/mock IM adapters with field-patch semantics and add shared contract tests for normalized routing and connector mutation results.
- [x] 1.7 Update the IM settings form to send edited fields only, clear submitted plaintext secrets on success or failure, retain safe non-secret edits when useful, and replace editable routing state with the normalized save result.
- [x] 1.8 Add frontend regression tests for partial credential replacement, normalized routing readiness, safe error behavior, and preservation of omitted configured fields.
- [x] 1.9 Run the focused communications, credential, adapter, and IM settings tests and keep `openspec validate harden-im-runtime-reliability --strict` passing before phase 2.

## 2. Event-driven completion and global limits

- [x] 2.1 Add session-runtime tests for exactly-once completed, failed, and cancelled notifications, terminal-before-receive races, dropped receivers, and registration cleanup.
- [x] 2.2 Extend the internal Agent/session execution boundary to create a one-shot terminal receiver before provider launch and complete it only after terminal message state is persisted.
- [x] 2.3 Replace IM `spawn_blocking` completion polling with the internal terminal receiver and remove the fixed 100 ms polling implementation after all callers migrate.
- [x] 2.4 Add deterministic runtime-manager tests for per-chat FIFO order, bounded total pending admission, bounded concurrent generations, localized busy responses, and capacity release on every terminal or rejected path.
- [x] 2.5 Implement centralized global pending reservations and an active-generation semaphore while preserving the existing per-chat queue bound and cross-chat concurrency.
- [x] 2.6 Replace one-task-per-message lane handling with bounded lane workers or equivalent bounded ownership so excess distinct chat ids cannot create unbounded tasks or completion receivers.
- [x] 2.7 Implement generation-safe idle lane reclamation and add race tests proving cleanup cannot remove a concurrently reused lane.
- [x] 2.8 Add a stress-oriented test that submits many distinct chat ids and asserts bounded active generations, pending work, worker count, completion registrations, and retained lanes.
- [x] 2.9 Run focused session-runtime and communications concurrency tests and keep strict change validation passing before phase 3.

## 3. Transactional connector lifecycle

- [x] 3.1 Add concurrency tests for overlapping save, enable, disable, restart, test, clear, authorization-persist, startup, and shutdown operations on the same and different connectors.
- [x] 3.2 Introduce a per-connector asynchronous lifecycle coordinator that serializes same-kind mutations while leaving unrelated connectors responsive.
- [x] 3.3 Implement candidate build and validation before stopping a live connector, with zeroizing snapshots and compensating restoration for credential or SQLite persistence failure.
- [x] 3.4 Replace separate runtime registration and start calls with one coordinated stop-and-replace operation and add tests proving repeated or concurrent starts cannot orphan workers.
- [x] 3.5 Restore the prior enabled configuration and runtime after replacement startup failure, and record distinct redacted primary and rollback outcomes when compensation fails.
- [x] 3.6 Make connection tests use isolated ephemeral adapters without stopping or replacing enabled inbound runtimes, including timeout and failed-test coverage.
- [x] 3.7 Make saved-connector startup and application shutdown attempt every connector and return or log connector-scoped safe outcomes instead of stopping at the first failure.
- [x] 3.8 Serialize WeChat authorization persistence with connector lifecycle mutations and add tests for concurrent poll, cancel, reauthorization, and confirmed-credential races.
- [x] 3.9 Run focused lifecycle, transport, operation-log, and authorization tests and keep strict change validation passing before phase 4.

## 4. Secondary performance and observability

- [x] 4.1 Move deduplication retention out of inbound claim handling into bounded startup and scheduled maintenance, with tests for throttling, retention cutoff, and shutdown.
- [x] 4.2 Add expiry-aware, single-flight access-token caches for Feishu and DingTalk with safety skew, authentication invalidation, and concurrent-send tests.
- [x] 4.3 Add an additive SQLite migration for WeChat reply-context last-used metadata and implement incremental per-chat secure-context migration with restart, retention, and rollback tests.
- [x] 4.4 Add connector-appropriate pacing for unexpectedly immediate successful empty polling responses while preserving normal long polling and responsive shutdown.
- [x] 4.5 Add redacted malformed-event diagnostics and bounded connector-specific acknowledgement/checkpoint behavior without logging frames, external identifiers, prompts, or responses.
- [x] 4.6 Extend the typed IM service with lifecycle subscription and unsubscribe behavior, implement validated Tauri events and deterministic Web/mock updates, and add adapter conformance tests.
- [x] 4.7 Update the IM settings page to apply generation-aware live lifecycle updates, ignore stale generations, clean up subscriptions, and retain manual refresh as recovery behavior.
- [x] 4.8 Add performance assertions or instrumentation tests covering completion latency, SQLite reads per completed IM message, token refresh count, dedup cleanup frequency, secure-context update scope, and idle lane retention.
- [x] 4.9 Update `src-tauri/ARCHITECTURE.md` and `docs/architecture/im-connectors-smoke.md` with the completion signal, capacity limits, lifecycle transaction, migration, status-update, and packaged-app verification behavior.
- [x] 4.10 Run frontend verification: `npm run lint`, `npm run test`, and `npm run build`.
- [x] 4.11 Run native verification: `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo clippy --manifest-path src-tauri/Cargo.toml`.
- [x] 4.12 Run `openspec validate harden-im-runtime-reliability --strict` and `openspec validate --specs --strict`, then record phase-by-phase implementation verification results in the change artifacts.
