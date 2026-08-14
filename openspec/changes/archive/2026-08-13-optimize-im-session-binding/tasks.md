## 1. Native Persistence and Domain Model

- [x] 1.1 Add failing Rust tests for pairing expiry, single use, connector scoping, session cardinality, explicit replacement, pause state, and legacy binding migration.
- [x] 1.2 Add additive SQLite schema and migration support for hashed pairing intents, managed binding state, notification preference, secure delivery references, timestamps, and bounded maintenance indexes.
- [x] 1.3 Migrate existing `im_session_bindings` as active managed bindings, retaining unexpected duplicate session bindings as paused records and preserving session source metadata.
- [x] 1.4 Extend communications domain models with validated pairing intents, safe binding views, lifecycle mutations, and replacement confirmation semantics.
- [x] 1.5 Extend repository ports and SQLite implementations for pairing consumption, binding queries and mutations, idempotent notification delivery, cascade cleanup, and bounded retention.
- [x] 1.6 Store and remove platform delivery handles through the existing credential adapter while keeping raw external identifiers out of SQLite and logs.

## 2. Native Routing and Delivery

- [x] 2.1 Add failing application tests proving configured connectors start and reconnect without global Agent or project routing.
- [x] 2.2 Remove global routing validation from connector enable, restart, and startup paths while retaining legacy routing data for rollback compatibility.
- [x] 2.3 Implement desktop-initiated pairing creation, cancellation, expiry, retry, and transactional IM-command consumption before Agent routing.
- [x] 2.4 Route active bound text through the existing session execution configuration and return safe guidance for paused, stale, or unbound chats without creating sessions.
- [x] 2.5 Implement explicit replacement confirmation and enforce one active external chat per session plus one target session per connector/chat pair.
- [x] 2.6 Implement origin-aware opt-in terminal notifications with safe content, idempotent dispatch, loop prevention, and no duplicate notification for IM-originated turns.
- [x] 2.7 Route all new pairing, binding, migration, and delivery diagnostics through unified logging with safe codes and pre-persistence redaction.

## 3. Native Commands and Contracts

- [x] 3.1 Add serializable DTOs and mappers for safe binding views, pairing results, lifecycle preferences, and replacement confirmation without exposing external identities or delivery handles.
- [x] 3.2 Add one-file-per-command Tauri handlers for listing a session binding, beginning and cancelling pairing, pausing and resuming, toggling notifications, confirming replacement, and removing a binding.
- [x] 3.3 Register the new communications commands and add command contract tests for validation, safe errors, authorization boundaries, and generation-aware state.
- [x] 3.4 Update generated Tauri contracts or schemas required by the registered command surface and verify no secret fields enter generated frontend payloads.

## 4. Frontend Service Boundary

- [x] 4.1 Add failing TypeScript contract tests for binding and pairing schemas, including rejection of raw external identity and credential fields.
- [x] 4.2 Extend `ImService` with typed session-binding operations and generation-aware subscriptions while removing routing defaults from connector readiness semantics.
- [x] 4.3 Implement all new operations in the Tauri IM adapter without adding direct `invoke()` calls to React components.
- [x] 4.4 Implement deterministic equivalent pairing and binding transitions in the Web/mock adapter and label simulated behavior appropriately.
- [x] 4.5 Update service contract parity checks so native and Web adapters cannot drift.

## 5. Settings IM Experience

- [x] 5.1 Add failing component tests proving connector enablement no longer requires an Agent or project and the routing form is absent.
- [x] 5.2 Replace global routing controls with localized connector-focused guidance and an action that leads to an eligible existing session.
- [x] 5.3 Preserve connector credential, authorization, access, test, lifecycle, health subscription, and safe field-patching behavior in both visual styles.

## 6. Session IM Experience

- [x] 6.1 Add failing hook and component tests for unbound, pairing, expired, bound, paused, connector-unavailable, replacement, notification, and removal states.
- [x] 6.2 Add a reusable session IM state hook backed only by `ImService`, with cleanup for subscriptions, pairing cancellation, and plaintext code removal.
- [x] 6.3 Add an IM tab to the session information panel showing configured connector health, guided pairing, safe binding metadata, and lifecycle controls.
- [x] 6.4 Add the equivalent responsive session-action entry for layouts where the information panel is hidden.
- [x] 6.5 Add accessible confirmation surfaces for replacement and removal and status announcements for asynchronous pairing and binding changes.
- [x] 6.6 Add synchronized zh-CN and en copy for settings guidance, pairing instructions, binding states, notification semantics, confirmations, accessible names, and safe errors.
- [x] 6.7 Verify futuristic and minimal styles, keyboard operation, focus behavior, narrow layouts, and the physical 300-line limit for new production TypeScript files.

## 7. Integration, Migration, and Regression Tests

- [x] 7.1 Add Rust integration tests covering connector startup without routing, unbound guidance, pairing interception, legacy binding reuse, stale-session cleanup, and final response delivery.
- [x] 7.2 Add tests proving session Agent, project/worktree, model, permission, history, and provider continuity are unchanged by binding operations.
- [x] 7.3 Add security regression tests proving pairing codes, raw chat/user ids, delivery targets, credentials, prompts, responses, and diagnostics are not persisted or logged through unsafe fields.
- [x] 7.4 Add frontend integration tests covering settings-to-session navigation, live pairing completion, pause/resume, notification toggles, connector health changes, and destructive confirmations.
- [x] 7.5 Add Playwright coverage for the primary desktop-sized flow and responsive session-action flow using the deterministic Web/mock adapter.
- [x] 7.6 Verify rollback compatibility by loading migrated active bindings through the compatibility repository path with legacy routing rows retained.

## 8. Required Validation

- [x] 8.1 Run `npm run lint:ci` and fix all reported issues.
- [x] 8.2 Run `npm run test` and `npm run test:coverage` and satisfy the enforced coverage thresholds.
- [x] 8.3 Run `npm run build`, `npm run coverage:policy:test`, `npm run version:unit:test`, and `npm run contracts:check`.
- [x] 8.4 Run `npx playwright test` for the UI behavior change and fix all failures.
- [x] 8.5 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [x] 8.6 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`.
- [x] 8.7 Run `cargo test --manifest-path src-tauri/Cargo.toml` and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 8.8 Run `openspec validate optimize-im-session-binding --strict` and `openspec validate --specs --strict`.

## 9. Verification Remediation

- [x] 9.1 Add failing hook and component tests for explicitly generating a replacement pairing code, then implement the localized retry action.
- [x] 9.2 Add failing native application tests for localized unbound, pairing, lifecycle, and completion messages, then inject locale-aware communications copy.
- [x] 9.3 Re-run targeted tests, required project validation, and OpenSpec verification after the remediation.
