## 1. Runner and Run Contracts

- [x] 1.1 Add provider-neutral Runner kinds, selections, descriptors, capabilities, launch/event/inspection models, stable error classes, and conformance-test helpers inside `agent_runtime` without adding a bounded context.
- [x] 1.2 Extend canonical Run domain, service inputs, serialized frontend contracts, and compatibility parsing with optional bounded Runner metadata and Runner-aware recovery evidence.
- [x] 1.3 Add the transactional SQLite migration, nullable Runner projection columns, composite Mission Control index, repository round-trip tests, rollback test, and legacy snapshot compatibility test.
- [x] 1.4 Extend the published Sessions API/gateway with current remote workspace and SSH binding projection required by Runner preparation without exposing repositories or credentials.
- [x] 1.5 Add architecture fitness coverage that permits only published cross-context Runner dependencies and rejects direct SSH infrastructure, permission infrastructure, process construction, or Tauri imports in domain/application code.

## 2. Compatible Local Runner

- [x] 2.1 Extract local process spawn, stdin, stream, handle, cancel, inspect, and cleanup behavior into `LocalRunner` behind the Runner port while keeping provider translation/parsing in the existing coordinator.
- [x] 2.2 Wire Local Runner through bootstrap and make omitted Runner selection normalize to Local before canonical Run creation.
- [x] 2.3 Add Local Runner conformance tests with fake process behavior for spawn, stdin, ordered events, natural exit, cancellation race, idempotent cleanup, and process-tree reaping.
- [x] 2.4 Add built-in CLI compatibility fixtures proving unchanged executable/args/cwd/prompt delivery, output parsing, resume ids, usage, errors, logs, and cancellation for existing Local execution.
- [x] 2.5 Preserve Agent terminal and Session archive/delete/quit behavior and add regressions proving renderer or route cleanup removes listeners without cancelling accepted native Runs.

## 3. Reused SSH Runner

- [x] 3.1 Publish a narrow lease-backed SSH execution facade for independent exec/PTY channels, keepalive, close, and pool inspection through `SshConnectionsApi`.
- [x] 3.2 Implement the bounded POSIX remote command encoder and owned-process wrapper with executable/argument/cwd/environment validation and property/negative tests.
- [x] 3.3 Implement `SshRunner` preparation using authoritative Session binding, profile revision, endpoint match, host trust, remote command availability, pool capacity, and no inherited local environment or Local fallback.
- [x] 3.4 Implement SSH spawn, stdin, bounded UTF-8 event streaming, opaque remote process reference, exit mapping, and independent channel cleanup using the existing pool.
- [x] 3.5 Implement bounded owned remote cancellation and escalation without closing unrelated pooled Terminal or Runner channels.
- [x] 3.6 Add fake SSH conformance tests for one provider on Local/SSH, transport single-flight reuse, independent channels, keepalive, disconnect, bounded inspect/reconnect, cancellation race, drain, and cleanup.

## 4. Permission, Recovery, Logging, and Resource Governance

- [x] 4.1 Extend permission evaluation inputs and tests with Runner kind, target/revision, execution action, and policy witness; require revalidation immediately before spawn.
- [x] 4.2 Implement Runner-scoped environment/secret admission and negative tests for stale authority, changed host key, missing credential, unauthorized secret, unsafe environment/cwd/argument data, and absence of secret bytes from DTOs, SQLite, logs, and telemetry.
- [x] 4.3 Persist Runner metadata before running, distinguish Runner and provider error classifications, and correlate bounded redacted Runner lifecycle diagnostics through the existing operations logging port.
- [x] 4.4 Implement startup reconciliation for Local `none` and SSH `inspect_only` recovery, including idempotent interrupted outcomes, current-authority checks, and explicit proof that no prompt, stdin, approval, question, tool, or destructive action is replayed.
- [x] 4.5 Integrate Runner admission and output/handle/reconnect/cleanup budgets with deterministic quota rejection and no-side-effect tests.
- [x] 4.6 Integrate graceful desktop quit ordering so Local trees and SSH channels are boundedly cancelled/cleaned while minimize and close-to-tray keep accepted Runs active.

## 5. Service Boundary and Runtime Adapters

- [x] 5.1 Add strict TypeScript Runner descriptors/selections/metadata and extend `SendMessageInput`, canonical Run, Mission Control query/summary, and safe error contracts with backward-compatible normalization.
- [x] 5.2 Extend `AgentService` with side-effect-free Runner discovery and selected execution, then add compile-time/service contract parity tests.
- [x] 5.3 Implement Tauri command/DTO/mapper/registry support using only assembled application services; add serialized compatibility and command-safe error tests.
- [x] 5.4 Implement deterministic Web/mock Local and SSH discovery, execution, disconnect/reconnect, cancellation, background page navigation, filtering, and application-exit limitation semantics without native claims.
- [x] 5.5 Extend canonical Run and Mission Control Web/Tauri contract normalization tests for explicit Runner metadata, legacy missing fields, invalid kinds/revisions, and Runner-versus-provider errors.

## 6. Runner Selection and Mission Control UI

- [x] 6.1 Add a compact accessible localized Runner selector to CLI Agent Run creation with Local default, eligible SSH choices, unavailable Docker/Sandbox status, loading/error/disabled states, and no runtime-specific component branches.
- [x] 6.2 Extend Mission Control with Local/SSH filter, semantic Runner badge, bounded host label, and disconnected/reconnecting/interrupted reason presentation while preserving owning-surface navigation.
- [x] 6.3 Add every Runner-visible string to all registered locale resources and pass locale key/interpolation/plural parity plus visible-text guardrails.
- [x] 6.4 Add Vitest coverage for selection defaults/revalidation, API Local-only state, SSH unavailable reasons, background Session navigation, Mission Control filter resets, badges, errors, keyboard/focus behavior, and narrow rendering.
- [x] 6.5 Add Playwright behavior and stable visual coverage for futuristic/minimal themes at desktop/narrow widths, including Local, SSH running, disconnected, interrupted, unavailable, loading, and error states.

## 7. Integration, Security, and Performance Evidence

- [x] 7.1 Add desktop integration coverage using fake Local and fake SSH provider processes through the real Tauri/service boundary, including same-provider Local/SSH Runs and Mission Control observation.
- [x] 7.2 Add security negative suites for command injection, path/environment controls, stale binding/trust/permission witnesses, credential leakage, unauthorized secret forwarding, and no silent fallback.
- [x] 7.3 Extend versioned performance datasets and harness metrics for 1/8/maximum mixed concurrent Runners, bounded handles/events/bytes/reconnects, SSH transport reuse, cleanup, and exactly-one-bound negative fixtures.
- [x] 7.4 Run dedicated Windows Runner benchmark evidence and record bounded provenance/metrics without treating wall-clock results as shared-CI gates.
- [x] 7.5 Verify migration from legacy databases, current databases, legacy serialized clients, and rollback-to-Local selection while preserving Sessions, Runs, messages, SSH profiles/trust, operations, logs, and observability records.

## 8. Required Validation and Delivery Evidence

- [x] 8.1 Run `npm run lint:ci`, `npm run test`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, `npm run contracts:check`, and `npm run build`.
- [x] 8.2 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 8.3 Run `npx playwright test`, inspect the required futuristic/minimal desktop/narrow visual artifacts, and record UI behavior/accessibility/visual results.
- [x] 8.4 Run `npm run desktop:unit:test` and `npm run test:desktop`; report Windows, macOS, and Linux Desktop Smoke only as actually run using `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN`.
- [x] 8.5 Run the Runner security negative suite and versioned deterministic/dedicated performance commands, and record migration, resource, cleanup, and benchmark evidence.
- [x] 8.6 Run `openspec validate --specs --strict` and `openspec validate add-agent-runner-abstraction --strict`, reconcile implementation against every proposal/spec/task acceptance scenario, and write the implementation verification report before archive.
