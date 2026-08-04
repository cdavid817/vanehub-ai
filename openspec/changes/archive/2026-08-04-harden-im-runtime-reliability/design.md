## Context

The IM runtime already normalizes five platform protocols, durably deduplicates inbound events, binds external chats to native sessions, runs Agent generations, and delivers final responses. The reliability audit found that several implementation choices do not preserve the intended contracts under routing changes, partial credential edits, concurrent lifecycle commands, sustained traffic, or protocol drift.

Today the IM route loads current global defaults even for an existing binding, completion waits occupy blocking workers and poll SQLite every 100 ms, each accepted message creates an asynchronous task, per-chat lanes remain in memory indefinitely, and lifecycle mutations span the credential store, SQLite, and runtime without one serialized rollback boundary. The settings page also has no runtime-neutral status subscription and can retain pre-normalized routing form values.

The change crosses React services, Tauri and Web/mock adapters, the native communications context, session/Agent runtime APIs, SQLite maintenance, the operating-system credential store, and platform transports. Secrets must remain write-only and redacted, existing databases and credential records must remain usable, and React components must not import Tauri APIs.

## Goals / Non-Goals

**Goals:**

- Preserve existing IM session bindings across global routing changes.
- Make partial credential edits safe, complete, validated, and recoverable.
- Deliver exactly one terminal completion to an IM job without periodic database polling.
- Bound active generations, total admitted pending work, and retained per-chat lane state.
- Serialize lifecycle mutations per connector and restore the previous usable state after a failed mutation.
- Keep connection testing isolated from an enabled inbound runtime.
- Make protocol skips observable without persisting sensitive payloads.
- Reduce recurring authentication, keyring, and SQLite maintenance overhead.
- Keep desktop and Web/mock IM service contracts behaviorally aligned.

**Non-Goals:**

- Adding connectors, group chats, attachments, streaming partial replies, or multi-user collaboration.
- Changing the five stable connector ids or exposing stored secrets to the frontend.
- Replacing SQLite, the operating-system credential store, or the shared Agent execution path.
- Guaranteeing outbound delivery after a process crash; durable outbound outbox work remains separate.
- Introducing a new frontend state-management library or platform SDK in React.

## Decisions

### 1. Existing bindings own their execution configuration

Global routing defaults will be consulted only when a binding needs a new session. Resolving an existing binding returns its session id, and Agent execution loads the Agent, project, provider continuity, and chat configuration from that bound session. It will not compare the bound Agent id with the current default Agent id.

This preserves the meaning of a dedicated session and prevents a settings change from invalidating established chats. Resetting bindings remains the explicit operation for moving chats to new defaults.

Alternative considered: update every bound session when defaults change. This was rejected because it destroys provider continuity and contradicts the existing binding contract.

### 2. Credential updates are typed patches applied to a validated candidate

Connector field metadata will distinguish non-secret configuration from secret fields. Non-secret values such as application ids and robot ids will be persisted in `public_config`; secret values remain in the operating-system credential store. An update will merge supplied fields with the stored configuration, construct the complete candidate, and validate every required field before stopping or replacing a runtime.

The existing command shape can remain compatible by interpreting the optional `credentials` map as a patch. Native code, not React, owns merging with stored secret values. A successful migration or save rewrites legacy all-in-one credential payloads into the split representation. Migration is idempotent, and the legacy payload remains readable until the split write succeeds.

Alternative considered: require users to re-enter all fields for every edit. This was rejected because secrets are intentionally write-only and the UI cannot safely reconstruct a complete credential set.

### 3. Completion is an internal one-shot signal registered before launch

The shared Agent execution API will expose a native submission result containing the assistant message identity and a one-shot terminal receiver registered before the provider process can reach a terminal state. Message terminal persistence occurs before the signal is completed. The receiver carries completed, failed, or cancelled state exactly once.

The implementation will also support a terminal-state fast path so attaching after persistence cannot wait indefinitely. Dropped or timed-out receivers are removed without retaining senders. IM will no longer hold a `spawn_blocking` worker that repeatedly queries `runtime_message`.

Alternative considered: increase the SQLite polling interval. This reduces load but retains latency, blocking-worker consumption, and race complexity, so it was rejected.

### 4. Admission is bounded globally and serialized per chat

Admission will reserve from a bounded total pending budget before creating a new lane or worker. Each chat retains its existing FIFO capacity and runs one lane worker. A global semaphore limits concurrent Agent generations across chats. If either the per-chat or global pending capacity is exhausted, the connector sends the localized busy response after durable deduplication and does not launch an Agent.

Lane entries use generation-safe cleanup: after their queue drains and no worker or reservation remains, the exact idle lane instance is removed from the map. This prevents a cleanup race from removing a newly reused lane.

Alternative considered: rely only on Agent-runtime process limits. This was rejected because IM tasks, completion receivers, queue state, and lane entries consume resources before an Agent process is admitted.

### 5. Connector lifecycle mutations use a per-kind coordinator

Save, enable, disable, restart, clear, test, authorization persistence, startup, and shutdown operations for the same connector will execute through one per-kind asynchronous coordinator. Different connectors remain independent.

Mutation flow is:

1. Load the previous configuration, credential, registered adapter, and lifecycle state.
2. Build and validate the complete candidate without modifying the live runtime.
3. Persist candidate credential and configuration with compensating snapshots.
4. Stop and atomically replace the registered runtime only when required.
5. If persistence or startup fails, restore the prior credential/configuration and restart the prior enabled runtime.
6. Record the primary and rollback outcomes using safe codes.

The runtime manager will expose an atomic replace/start operation rather than public `register` followed by `start`. Connection tests use an ephemeral adapter and never stop, register, or replace an enabled runtime. Startup and shutdown attempt every connector and aggregate safe failures rather than returning after the first connector error.

Alternative considered: one global lifecycle mutex. This was rejected because a slow Telegram test must not block a Feishu disable or settings refresh.

### 6. Protocol admission distinguishes unsupported events from malformed events

Known group and unsupported-content events remain safely acknowledged and skipped. A payload that cannot be normalized is classified separately, emits a redacted diagnostic containing connector, operation, and safe code only, and follows a connector-specific bounded acknowledgement/checkpoint policy that prevents both silent loss and infinite retry storms. Raw frames, chat ids, user ids, message text, and reply contexts never enter diagnostics.

### 7. Maintenance and transport caches are incremental and bounded

- Deduplication cleanup runs during startup and on a throttled maintenance schedule, not on every inbound claim.
- Feishu and DingTalk access tokens are cached until their provider expiry minus a safety margin, with single-flight refresh and invalidation after authentication rejection.
- WeChat reply contexts are stored as per-chat secure entries addressed by a stable hash. Non-secret last-used metadata is stored additively in SQLite so expired entries can be removed without reading and rewriting one unbounded credential blob.
- Successful empty polling responses use connector-appropriate pacing so a provider that returns immediately cannot create a hot loop.

### 8. Lifecycle status updates stay behind the frontend service boundary

The typed `ImService` contract will expose lifecycle status subscription with an unsubscribe handle. The Tauri adapter owns native event listening and schema validation. The Web/mock adapter emits deterministic simulated transitions and stores no secrets. React subscribes through the service, applies generation-aware updates, and unsubscribes on unmount; it does not import Tauri event APIs.

Save-routing and connector mutations return normalized results. The page replaces both saved and editable state with the normalized routing response, avoiding false “routing incomplete” state caused by path or whitespace normalization.

## Risks / Trade-offs

- [Cross-store atomicity cannot use one database transaction] → Keep zeroizing snapshots, order mutations explicitly, execute compensating rollback, and surface rollback failure as a distinct safe diagnostic while leaving the connector disabled rather than running unknown credentials.
- [Completion can race with receiver registration] → Register the one-shot before provider launch and retain a persisted terminal fast path.
- [Global limits may reject bursts that previously consumed unbounded resources] → Use localized busy responses, expose safe capacity diagnostics, and keep constants centralized and testable.
- [Lane cleanup can race with new arrival] → Remove only the same idle lane generation while holding the lane-map synchronization boundary.
- [Token caching can reuse an invalid token] → Apply expiry skew, invalidate on authentication responses, and single-flight refresh.
- [Legacy WeChat context migration touches sensitive data] → Migrate incrementally inside the native credential boundary, never log payloads, delete the legacy blob only after all writes succeed, and support restartable migration.
- [Runtime status events can arrive out of order] → Carry connector generation and timestamp and ignore stale frontend updates.

## Migration Plan

1. Phase 1 — correctness: add regression tests, separate new-binding defaults from bound-session execution, introduce credential patch merging and idempotent legacy credential splitting, and synchronize normalized frontend routing state.
2. Phase 2 — event-driven completion and limits: add the internal one-shot completion API, migrate IM execution off polling, add global pending/active limits, and implement race-safe idle lane reclamation.
3. Phase 3 — lifecycle transactionality: introduce per-kind coordinators, candidate validation, compensating rollback, atomic runtime replacement, isolated connection tests, and failure-isolated startup/shutdown.
4. Phase 4 — secondary performance: schedule dedup cleanup, add token caches and polling pacing, migrate WeChat contexts incrementally, add safe malformed-event diagnostics, and add runtime-neutral lifecycle status updates.
5. Run focused unit and integration tests after each phase. Before completion, run all project verification commands and `openspec validate harden-im-runtime-reliability --strict`.

Rollback is phase-aware. Code must remain able to read legacy credential and WeChat context formats until the change is fully deployed. Additive SQLite metadata can remain unused after rollback. A failed runtime migration keeps the legacy secure record and previous configuration authoritative.

## Open Questions

- The initial global active-generation and total-pending limits should be selected from deterministic stress tests and documented as centralized constants; they are not user-configurable in this change.
- Provider-specific minimum successful-poll pacing values should be confirmed against existing transport behavior during implementation without changing normal long-poll semantics.

## Implementation verification

- Phase 1 — correctness fixes: focused communications credential/routing tests and frontend IM form/adapter contract tests passed; strict change validation remained valid before phase 2.
- Phase 2 — event-driven completion and global limits: Agent Runtime completion tests, the three completion-registry race/cleanup tests, and communications FIFO/capacity/lane stress tests passed; the fixed 100 ms SQLite polling module was removed.
- Phase 3 — transactional lifecycle: 68 focused communications tests passed, including same-kind serialization, cross-kind responsiveness, replacement rollback, isolated connection testing, all-connector startup/shutdown, and WeChat authorization races.
- Phase 4 — secondary performance and observability: 75 focused communications tests and 21 focused frontend IM tests passed, covering token single-flight, maintenance throttling, secure-context migration/retention/rollback, polling pacing, bounded diagnostics, and lifecycle subscriptions.
- Post-verification critical remediation: 81 focused communications tests passed after connector clear was changed to stop before deleting credentials, tracked per-chat WeChat secure contexts were purged in retry-safe batches, and deduplication cleanup was capped at 512 rows per maintenance run.
- Final frontend gates: `npm run lint`, `npm run test` (413 passed), and `npm run build` passed.
- Final native gates: `cargo test --manifest-path src-tauri/Cargo.toml` (1062 passed, 3 ignored, plus 12 architecture tests), `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo clippy --manifest-path src-tauri/Cargo.toml` passed without warnings.
- Final specification gates: `openspec validate harden-im-runtime-reliability --strict` and `openspec validate --specs --strict` passed; all 82 main specs validated.
