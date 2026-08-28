## 1. Session Access Model and Migration

- [x] 1.1 Add the `ImSessionAccess` domain/view contracts and connector-scoped repository operations with missing state normalized to disabled.
- [x] 1.2 Add the append-only `im_session_connector_access` SQLite migration with session cascade deletion, timestamps, connector validation, and no secret or external-identity fields.
- [x] 1.3 Backfill enabled Feishu access only for sessions with an existing Feishu binding, preserving manually paused binding state and leaving unbound/new sessions disabled.
- [x] 1.4 Add empty, current, legacy, repeated-migration, backfill, cascade-delete, and rollback-safe database tests.

## 2. Native Authorization and Service Boundary

- [x] 2.1 Extend the communications repository port, application API, and command mapper with get/set session connector access operations returning normalized state.
- [x] 2.2 Add and register `set_im_session_access` at the Tauri command boundary with validated session id, stable connector id, `Result<T, String>` error mapping, and unified redacted operation logging.
- [x] 2.3 Enforce enabled Feishu access before pairing-intent creation and re-check it before pairing consumption so disabling a session cannot be bypassed with a previously issued code.
- [x] 2.4 Enforce enabled Feishu access after binding resolution and before Agent execution admission, failing closed for missing state or repository errors without exposing session or external-chat metadata.
- [x] 2.5 Re-check session access before optional desktop completion notifications while preserving the final response for an IM turn already admitted before disable.
- [x] 2.6 Serialize session access mutation and inbound admission sufficiently to guarantee that a completed disable prevents subsequently admitted Feishu work.
- [x] 2.7 Add native tests for default denial, enabled pairing, disable-before-consume, disabled inbound, re-enable, manual-pause preservation, connector-health gating, notification suppression, repository failure, and disable/inbound races.

## 3. Single-Agent and Multi-Agent Routing

- [x] 3.1 Pass Feishu origin metadata and unmodified normalized text through the existing shared Agent execution port without adding seat parsing to the communications or transport layers.
- [x] 3.2 Verify single-Agent Feishu turns reuse the bound session's stable Agent id, project/worktree configuration, provider continuity, and one terminal outbound response.
- [x] 3.3 Verify multi-Agent Feishu turns reuse stable seat identities, line-leading mention routing, current-owner/first-seat fallback, and bounded serial Agent handoffs.
- [x] 3.4 Add a typed safe outcome for missing, removed, or unavailable mentioned seats and ensure communications returns localized valid-seat guidance without silently rerouting.
- [x] 3.5 Add integration tests for explicit seat mention, no mention, invalid seat, removed seat, cross-Agent handoff, handoff limit, duplicate event, terminal-only delivery, oversized Unicode-safe chunking, and outbound failure without Agent rerun.

## 4. Frontend Contracts and Runtime Adapters

- [x] 4.1 Extend strict IM schemas and `ImService` with connector-scoped session access plus access state in `ImSessionBindingView`, without using `any` or weakening validation.
- [x] 4.2 Implement the Tauri IM adapter methods and parsing so React remains free of direct Tauri `invoke()` and event APIs.
- [x] 4.3 Implement deterministic per-session/per-connector access in the Web/mock adapter with default-off, mutation, binding, pause, reset, and session-isolation semantics matching desktop behavior.
- [x] 4.4 Update `useSessionImState` to load and mutate access safely across session switches, stale responses, pending mutations, pairing expiry, and lifecycle events.
- [x] 4.5 Add contract, Tauri adapter, Web/mock adapter, and hook tests covering malformed native responses, default-off state, session isolation, enable/disable, re-enable, manual pause, stale requests, and error recovery.

## 5. Information Panel Experience

- [x] 5.1 Add a keyboard-operable, accessible IM switch to the selected session's information panel and keep pairing/binding controls unavailable while it is off.
- [x] 5.2 Limit the first enabled connector choice in the session surface to Feishu while leaving global configuration and health management for the other connectors unchanged.
- [x] 5.3 Add confirmation before disabling a bound session and render distinct effective reasons for session opt-out, manual binding pause, and connector lifecycle unavailability.
- [x] 5.4 Restore native persisted state after restart, preserve responsive/narrow-layout access, and keep component production files within the 300-line limit by extracting focused presentation pieces where needed.
- [x] 5.5 Add synchronized localized copy and accessible names in all maintained locale resources without exposing credentials, external ids, prompts, or message content.
- [x] 5.6 Add component and Playwright coverage for default off, keyboard operation, successful enable, failed enable, bound-session confirmation, disabled controls, manual pause preservation, session switch, Web/mock behavior, and responsive entry.

## 6. Deterministic Feishu Desktop Fixtures

- [x] 6.1 Add a `desktop-e2e`-only Feishu fixture assembly using recorded sanitized protocol events, the deterministic CLI Agent fixture, a fake connected transport, and an outbound ledger containing safe sequence/status metadata only.
- [x] 6.2 Add narrowly scoped fixture commands for setup, event injection, fault/recovery control, and sanitized evidence; feature-gate both implementation and command registration and require a layer-specific runtime flag.
- [x] 6.3 Add architecture tests proving Feishu fixture code, commands, permissions, and globals are absent from normal production/release builds.
- [x] 6.4 Add `tests/desktop/wdio.feishu-im.conf.mjs`, isolated specs/evidence directories, `test:desktop:feishu-im`, and a `test-desktop.mjs` layer result using existing process ownership and cleanup infrastructure.
- [x] 6.5 Drive the rendered desktop information panel with WebdriverIO and verify new-session default-off state, native persistence, enablement, fixture pairing, single-Agent round trip, and real relaunch restoration.
- [x] 6.6 Extend WebdriverIO coverage to multi-Agent mentioned/default/invalid-seat routing, duplicate and malformed events, disabled sessions, disable races, reconnect, oversized output, outbound failure, and clean shutdown.
- [x] 6.7 Verify screenshots, WDIO diagnostics, result JSON, fixture ledger, and collected unified logs contain no credential, external identity, prompt, Agent response, or raw protocol payload.

## 7. Live Feishu Qualification

- [x] 7.1 Add an explicit opt-in live qualification entry point that reports `BLOCKED` or `NOT RUN` when the dedicated tenant, App ID/App Secret, permissions, long-connection subscription, or direct test chat is unavailable.
- [x] 7.2 Accept live credentials only at runtime, enter them through the normal write-only settings path, suppress secret-entry screenshots/logging, use isolated app data and credential references, and clear run-owned connector credentials during cleanup.
- [x] 7.3 Document the minimum Feishu bot/message permissions and operator setup for direct-message receipt and reply using official Feishu references and least-privilege guidance.
- [x] 7.4 When the user supplies the live environment, execute authentication, connection lifecycle, direct receipt, duplicate delivery, single-Agent response, multi-Agent routing, chunking, disable/re-enable, restart, and invalid-credential scenarios and record each actual result separately from fixture evidence.
  - 2026-08-27 live evidence: `test-results/desktop-live/2026-08-27T06-40-42-800Z-b63cd2b6/`.
  - `PASSED`: credential isolation, authentication, connection start/restart, connector disable/re-enable, direct receipt, single-Agent response, oversized chunking, mentioned/default/invalid-seat multi-Agent routing, desktop restart, invalid credential rejection, and run-owned credential cleanup.
  - `BLOCKED`: duplicate delivery (`feishu-platform-retry-not-observed`); the live tenant did not produce an observable platform retry, while deterministic deduplication remains covered separately.
  - `FAILED`: session disable/re-enable (`webdriver-command-timeout`); no inbound was admitted during the measured disabled interval, and the operator later observed the re-enabled Agent reply after the scenario's wait window had expired.

## 8. Focused and Full Verification

- [x] 8.1 Run focused frontend IM contract, service, hook, component, localization, architecture, and Playwright tests and fix every regression.
- [x] 8.2 Run focused Rust communications schema, repository, application, routing, Feishu transport, logging-redaction, race, and production-boundary tests and fix every regression.
- [x] 8.3 Run `npm run test:desktop:feishu-im` against a newly built WebdriverIO/Tauri desktop test artifact and retain the run-scoped deterministic evidence.
- [x] 8.4 Run `npm run lint:ci`, `npm run test`, `npm run build`, `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `npm run native:panic:check`, `cargo test --workspace`, and `openspec validate --specs --strict`.
- [x] 8.5 Run `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, `npm run contracts:check`, `npm run architecture:check`, `npx playwright test`, `npm run desktop:unit:test`, and `npm run test:desktop` because this change touches UI behavior, runtime adapters, Tauri IPC, native persistence, and desktop verification.
- [x] 8.6 Run `openspec validate add-session-im-toggle --strict`, record every verification command and actual outcome in the change artifacts, and leave live Feishu results explicitly `BLOCKED` or `NOT RUN` until the required user-supplied environment is available.

### Verification record (2026-08-27, Windows)

- Focused frontend: six IM contract/service/hook/component files passed 50 tests; localization passed 29 tests; `npx playwright test tests/e2e/im-settings.spec.ts` passed 3 tests.
- Focused native: communications passed 109 tests, Feishu passed 11 tests, targeted runtime/architecture/boundary tests passed, and `cargo check -p vanehub-ai --features desktop-e2e` passed.
- `npm run lint:ci`: `PASSED`.
- `npm run test`: `PASSED` (313 files, 1605 tests).
- `npm run build`: `PASSED` (16 lazy chunks; main static closure 140.2 KiB gzip).
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`: `PASSED`.
- `cargo check --workspace`: `PASSED`.
- `cargo clippy --workspace --all-targets -- -D warnings`: initially found two actionable diagnostics; both were fixed without allowances, then `PASSED`.
- `npm run native:panic:check`: `PASSED`.
- `cargo test --workspace`: initial contract assertions were corrected; a later full run had one unrelated Windows process-cleanup timing failure that passed in isolation; the final complete run `PASSED`.
- `openspec validate --specs --strict`: `PASSED` (137 specs).
- `npm run test:coverage`: `PASSED` (1605 tests; statements 71.86%, branches 67.97%, functions 66.89%, lines 75.66%).
- `npm run coverage:policy:test`: `PASSED` (5 tests).
- `npm run version:unit:test`: `PASSED` (9 tests).
- `npm run contracts:check`: `PASSED` (3 files, 16 tests).
- `npm run architecture:check`: `PASSED` (repository checks plus 50 native architecture tests).
- `npx playwright test`: `FAILED` with 165/166 passing because `/help` output missed its timeout under full-suite load; the exact failed test then `PASSED` in isolation. All IM and multi-Agent browser scenarios passed in the full run.
- `npm run desktop:unit:test`: `PASSED` (44 tests).
- `npm run test:desktop:feishu-im`: `PASSED` on a newly built artifact (4 files; deterministic evidence `test-results/desktop/2026-08-27T09-18-15-045Z-80f46d79/`).
- `npm run test:desktop`: `FAILED` after early domain specs passed because the shared WebDriver session became persistently unresponsive in basic element queries; the run was stopped after the same infrastructure timeout repeated. Host-mutating update, bulk CLI, and live-keyring scenarios reported their designed `BLOCKED` status.
