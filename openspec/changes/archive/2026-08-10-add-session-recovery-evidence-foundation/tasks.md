## 1. Recovery Domain and Persistence Schema

- [x] 1.1 Add typed session recovery status, recovery decision, reason-code, evidence-reference, and immutable recovery-report domain models with exhaustive serialization tests.
- [x] 1.2 Add an additive SQLite migration for session recovery/revision fields, active execution-run ownership, the next-message-sequence allocator, message sequence/run correlation, and session-owned recovery reports using the next migration number available on current main.
- [x] 1.3 Backfill historical message sequences deterministically by session, creation timestamp, and stable message id; initialize each session allocator and add the `(session_id, session_sequence)` uniqueness constraint/index.
- [x] 1.4 Add migration fixtures covering clean legacy sessions, equal message timestamps, orphan active sessions, provider resume metadata, API tool-use snapshots, and Plan/Loop-owned sessions.
- [x] 1.5 Explicitly configure and test the SQLite synchronous durability level used by recovery-critical commits while retaining WAL, foreign keys, and bounded busy handling.
- [x] 1.6 Extend session/message rows, records, mappers, repositories, command DTOs, and Web normalized models with recovery metadata, revisions, sequence, and nullable execution-run correlation.
- [x] 1.7 Add repository tests proving recovery reports cascade with session deletion and historical execution-run ids remain null rather than being fabricated.

## 2. Durable Generation and Message Transactions

- [x] 2.1 Implement transactional allocation of one or more consecutive message sequence values without using an unlocked `MAX(sequence) + 1` query.
- [x] 2.2 Implement the generation-start transaction that conditionally claims a recovery-clean non-archived session, associates the execution run, inserts/correlates user and assistant messages, and advances lifecycle and revisions.
- [x] 2.3 Route desktop, API Agent, managed CLI, IM/headless, scheduled, Plan, Loop, and seat-handoff generation starts through the shared durable claim before provider/process execution.
- [x] 2.4 Implement the generation-terminal transaction that conditionally terminalizes the correlated assistant message, stores unique usage when present, clears only the matching active claim, and advances revisions.
- [x] 2.5 Preserve process-local generation handles only for cancellation/control and make their reservation subordinate to the durable database claim.
- [x] 2.6 Add concurrent-claim tests proving at most one same-session generation starts across pooled database connections while unrelated sessions remain isolated.
- [x] 2.7 Add transaction failure-injection tests proving partial start or terminal writes are not visible after reopening a file-backed database.

## 3. Evidence Collection and Recovery Decisions

- [x] 3.1 Define the session terminal-evidence port and bounded typed evidence model for session, message, operation, available tool activity, provider resume metadata, and live-handle presence.
- [x] 3.2 Implement consistent evidence reads keyed by session and known execution run without consulting unified logs or retention-bounded observability as authority.
- [x] 3.3 Implement a pure deterministic decision engine for completed, failed, cancelled, tool-free interruption, action-required ambiguity, quarantine-worthy corruption, and retry-later storage conditions.
- [x] 3.4 Detect cross-run and incompatible-terminal conflicts without selecting a winner from timestamps, message position, or source precedence.
- [x] 3.5 Classify unfinished legacy tool-use snapshots and opaque CLI/provider-internal tool activity as action-required while allowing conclusively tool-free partial responses to terminate cleanly.
- [x] 3.6 Add table-driven decision tests for API, managed CLI, provider-resume, missing telemetry, partial streaming, conflicting run ids, incomplete tools, and retryable storage evidence.
- [x] 3.7 Add property-style invariant tests proving terminal states never regress and equivalent evidence ordering produces the same decision.

## 4. Startup Recovery Coordinator

- [x] 4.1 Implement bounded candidate scanning and conditional transition into `reconciling` using captured state/history/recovery revisions.
- [x] 4.2 Implement conditional recovery publication that applies the session/message projection and one immutable report atomically only when the captured run and revisions still match.
- [x] 4.3 Make repeated coordinator passes idempotent and defer temporary database contention or unavailable evidence adapters without quarantining the session.
- [x] 4.4 Replace the existing unconditional orphan-to-failed mutation with the new coordinator while preserving partial content and provider runtime resume metadata.
- [x] 4.5 Split one-shot startup recovery from hourly archival/retention maintenance and exclude active, reconciling, action-required, and quarantined sessions from automatic archival.
- [x] 4.6 Reorder bootstrap so runtime/evidence adapters attach before ordinary session recovery, Plan/Loop projections run afterward, and recurring maintenance starts last.
- [x] 4.7 Persist safe recovery diagnostics through unified logging with session/run/report correlation and verify prompts, message bodies, tool payloads, commands, credentials, private paths, and raw provider errors are excluded.
- [x] 4.8 Add file-backed crash-point tests that discard all services, reopen SQLite, reconstruct the coordinator, and verify generation-claim, partial-stream, terminal-message, stale-revision, and duplicate-pass recovery.

## 5. Recovery Actions and Dependent Runtimes

- [x] 5.1 Implement revision-checked acknowledgement for action-required sessions that writes a new report, clears the recovery gate, retains the interrupted lifecycle/evidence, and performs no retry or tool repair.
- [x] 5.2 Reject acknowledgement for stale revisions and quarantined sessions with normalized safe errors and current recovery state.
- [x] 5.3 Change Plan recovery to consume the shared session/run terminal projection and create new attempt/session/run identities for explicit retry.
- [x] 5.4 Change Loop recovery to consume shared Worker/Verifier session projections and preserve paused recovery-required behavior for ambiguous child work.
- [x] 5.5 Correlate each serial multi-seat generation with its own execution run and retain optional round/parent-run context so only the active seat is interrupted.
- [x] 5.6 Add integration tests for ordinary sessions, Plan attempts, Loop roles, and multi-seat handoffs proving one shared evidence result is projected once without recent-message inference.

## 6. Native Commands, Service Contracts, and Events

- [x] 6.1 Add native APIs and one-command-per-file Tauri commands for reading bounded recovery summaries/reports and acknowledging action-required recovery with `Result<T, String>` or the existing command error boundary.
- [x] 6.2 Extend the typed `session:event` union with recovery invalidation variants carrying session id and revision, and emit them only after durable commit.
- [x] 6.3 Extend `src/services/agent-service.ts` with normalized recovery types, read/acknowledgement methods, and revisioned event variants without introducing `any`.
- [x] 6.4 Implement matching Tauri adapter methods/listeners with `invoke()` confined to `src/services/tauri-agent-client.ts`.
- [x] 6.5 Implement deterministic Web/mock recovery states, reports, acknowledgement, sending gates, and events without claiming native process or SQLite recovery.
- [x] 6.6 Add shared adapter contract tests proving desktop DTO mapping and Web/mock fixtures expose equivalent recovery fields, mutations, stale-revision errors, and event shapes.

## 7. Recovery-Aware Chat Experience

- [x] 7.1 Update the shared session admission logic and composer to allow clean failed/stopped sessions while blocking archived, reconciling, action-required, quarantined, or actively claimed sessions.
- [x] 7.2 Add accessible localized recovery-state presentation for reconciling, action-required, and quarantined sessions while keeping the existing transcript readable.
- [x] 7.3 Add the action-required acknowledgement confirmation flow with explicit text that it neither retries work nor proves whether an uncertain external effect occurred.
- [x] 7.4 Keep supported inspection/export surfaces available for quarantined sessions and remove stop controls when no live generation exists.
- [x] 7.5 Refresh authoritative state on initial load, revisioned session events, revision gaps, acknowledgement completion, and stale-mutation responses so event delivery is never the only state source.
- [x] 7.6 Add component tests for send/stop gating, partial-content preservation, acknowledgement, stale revision, missing event, and quarantined read-only behavior through service doubles.
- [x] 7.7 Add Playwright coverage for desktop-compatible recovery review and Web/mock parity without direct Tauri calls from React.

## 8. Verification and Governance

- [x] 8.1 Run `npm run lint:ci` and fix all reported frontend issues without adding ESLint exemptions.
- [x] 8.2 Run `npm run test`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 8.3 Run `npm run build` and `npx playwright test` because session send/stop/recovery UI behavior changes.
- [x] 8.4 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [x] 8.5 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- [x] 8.6 Run `cargo test --manifest-path src-tauri/Cargo.toml`, including file-backed migration, concurrency, transaction, and crash-reopen suites.
- [x] 8.7 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 8.8 Run `openspec validate add-session-recovery-evidence-foundation --strict` and `openspec validate --specs --strict`, then record the implementation verification results before archival.

## 9. Archival Blocker and Recovery Boundaries

- [x] 9.1 Persist recovery-critical operation terminal evidence across process restart and add a file-backed crash-reopen recovery test.
- [x] 9.2 Drain startup recovery candidates in bounded batches, retain retry-later candidates for an explicit in-process retry, and test more than one hundred candidates.
- [x] 9.3 Read bounded message evidence by the active execution run, detect conflicting unfinished run ownership, and test recovery with more than 256 historical messages.
- [x] 9.4 Stabilize the full Playwright run, then rerun the exact archival verification gates and update the implementation verification record.
- [x] 9.5 Classify deterministic persisted session/message evidence decoding failures as structural recovery evidence, quarantine the affected candidate without exposing raw payloads, and add a file-backed malformed-row regression test.
- [x] 9.6 Run one explicit in-process retry before Plan/Loop startup projection when the initial session recovery pass defers work, remove the late uncoordinated retry, and add ordering/trigger regression coverage.
- [x] 9.7 Stabilize the three reproducible Windows MCP relay socket and timing fixtures without changing production MCP behavior, rerun the full native test gate, and update the implementation verification record.

## 10. Performance Hardening

- [x] 10.1 Stop rebuilding the trigram message-search index for every streaming persistence flush and index streamed content once it reaches a terminal message state.
- [x] 10.2 Add query-plan-aligned indexes for run-scoped recovery evidence, unfinished cross-run witnesses, and startup recovery candidate scans.
- [x] 10.3 Restrict frontend recovery polling to transient reconciliation and invalidate only the authoritative recovery queries required by that fallback.
- [x] 10.4 Add migration, query-plan, streaming-index, and frontend polling regression coverage, then rerun the affected verification gates and record the results.
