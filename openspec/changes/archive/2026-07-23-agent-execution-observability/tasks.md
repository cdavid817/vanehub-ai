## 1. Dependency and Compatibility Foundation

- [x] 1.1 Verify the current Rust toolchain against one compatible `tracing`, OpenTelemetry API/SDK/OTLP, log bridge, and semantic-convention crate family, then record the pinned versions and GenAI/MCP schema version in the codebase.
- [x] 1.2 Build and document a tested Claude Code, Codex CLI, Gemini CLI, and OpenCode capability matrix for tool lifecycle events and invocation-scoped MCP configuration without launching interactive sessions during availability checks.
- [x] 1.3 Add the selected Rust telemetry dependencies with only the trace, metric, log, propagation, and OTLP protocol features required by the design.

## 2. Shared Execution Domain and Contracts

- [x] 2.1 Add Rust execution-run, trace/span identity, lifecycle, fidelity, capture-policy, source, link, safe-attribute, and paginated timeline domain types with validation tests.
- [x] 2.2 Define the native `ExecutionTelemetryPort` and deterministic capturing test adapter without exposing OpenTelemetry SDK types to application services.
- [x] 2.3 Extend observable Agent operation results with optional run and trace correlation while preserving existing operation ids, lifecycle, logs, results, errors, and serialization.
- [x] 2.4 Add shared Rust command DTOs and generated or verified TypeScript contracts for observability settings, run summaries, span summaries, events, fidelity, pagination, and typed errors.
- [x] 2.5 Add contract-drift and serialization tests covering absent legacy correlation, all fidelity values, bounded attributes, and pagination tokens.

## 3. SQLite Timeline and Retention

- [x] 3.1 Add an additive SQLite migration for execution runs, spans, bounded lifecycle events, links, settings, and the indexes required by run, session, status, and time-based queries.
- [x] 3.2 Implement the Rust execution timeline repository for transactional run/span start, idempotent update, terminal completion, and safe event append operations.
- [x] 3.3 Implement bounded paginated run and timeline queries that never return raw prompts, model output, tool payload bodies, headers, credentials, environment values, or unrestricted paths.
- [x] 3.4 Implement scheduled 1-to-90-day retention maintenance with a 30-day default and tests proving cleanup does not scan or delete on every emitted event.
- [x] 3.5 Add migration, repository round-trip, pagination, duplicate-event, crash-left-open-span, and retention tests using isolated database fixtures.

## 4. Native Telemetry Infrastructure

- [x] 4.1 Implement a composite telemetry adapter that maps domain records to `tracing`/OpenTelemetry spans, metrics, logs, and the local SQLite timeline using the pinned schema version.
- [x] 4.2 Configure W3C trace-context propagation, bounded asynchronous OTLP processors, resource attributes, sampling, short timeouts, dropped-data accounting, and bounded shutdown flushing.
- [x] 4.3 Implement metadata-only and bounded redacted-content mapping with centralized redaction before SQLite, unified logs, and OTLP sinks.
- [x] 4.4 Bridge unified diagnostic and operation logs to optional OpenTelemetry export while preserving the configured JSONL sink, page-level operation logs, levels, rotation, archival, and redaction.
- [x] 4.5 Add rate-limited local diagnostics for exporter failures and tests proving exporter errors do not recurse through the failed exporter or fail user operations.
- [x] 4.6 Register the telemetry service during Tauri bootstrap with disabled-export safe defaults and ensure application shutdown performs only a bounded flush.

## 5. Observability Settings

- [x] 5.1 Extend native settings persistence with local timeline enablement, OTLP enablement, validated endpoint and protocol, sampling ratio, 1-to-90-day retention, capture policy, and MCP relay enablement.
- [x] 5.2 Store optional OTLP authentication material through the native credential service and return only configured indicators or safe references through command contracts.
- [x] 5.3 Implement prospective settings snapshots so an active run keeps its original identity, sampling, capture, and relay policy after settings change.
- [x] 5.4 Add settings validation, upgrade-default, credential non-disclosure, active-run snapshot, and invalid-update rollback tests.

## 6. Task, Agent, and Process Instrumentation

- [x] 6.1 Create one execution run in the shared desktop, IM, and scheduled message execution entry point before prompt assembly and correlate existing source, session, messages, operation, stable Agent, and provider-session ids.
- [x] 6.2 Carry the opaque execution context through generation requests, coordination leases, process management, monitoring threads, event sinks, cancellation, and terminal persistence.
- [x] 6.3 Instrument prompt assembly and Agent invocation stages with safe status, duration, error classification, and pinned GenAI semantic attributes without recording prompt content by default.
- [x] 6.4 Instrument child-process construction, spawn, pid association, first visible output, cancellation initiator, monitoring failure, exit status, and terminal duration while excluding raw prompts and environment values.
- [x] 6.5 Record parent-Agent, delegation, attempt, and span-link metadata when provider events expose them, while treating independent retries as new linked runs.
- [x] 6.6 Add tests for successful, failed-before-spawn, failed-after-spawn, cancelled, empty-output, concurrent-session, background-thread, and exporter-unavailable generations.

## 7. Provider Tool Lifecycle Normalization

- [x] 7.1 Replace start-only tool parsing with a lifecycle-aware normalized event model keyed by stable provider call id and carrying phase, provider timestamp when available, safe status, and fidelity.
- [x] 7.2 Update Claude Code, Codex CLI, Gemini CLI, and OpenCode adapters to map supported start, completion, failure, duplicated, and out-of-order tool events without display-name-based merging.
- [x] 7.3 Create inferred tool spans for provider-reported calls, end unmatched calls as incomplete at Agent termination, and avoid fabricating success or duration when boundaries are missing.
- [x] 7.4 Preserve the existing chat tool-use block contract and UI behavior while adding execution correlation through service-layer metadata rather than component-level Tauri access.
- [x] 7.5 Expand committed provider fixtures and parser/application tests for complete, start-only, missing-id, missing-name, duplicate, out-of-order, failed, and concurrent tool calls.

## 8. MCP Native and Relay Observability

- [x] 8.1 Instrument existing MCP connection tests and VaneHub-native MCP operations with pinned method/transport attributes, operation/run correlation, duration, outcome, and bounded error classification.
- [x] 8.2 Implement an invocation-scoped stdio MCP relay that forwards protocol messages literally, preserves cancellation and timeout behavior, correlates each request, and omits payload bodies under metadata-only capture.
- [x] 8.3 Implement the supported HTTP MCP relay path with safe header handling, W3C propagation where available, session preservation, timeout/cancellation parity, and no redirect or credential leakage regressions.
- [x] 8.4 Add provider-adapter support for opt-in invocation-scoped VaneHub-managed MCP configuration without mutating user-global configuration.
- [x] 8.5 Return verified native, proxied, inferred, or opaque MCP observation capability for each stable Agent id and transport, and fall back without blocking Agent execution when relay support is disabled or unavailable.
- [x] 8.6 Add transparent-forwarding, JSON-RPC literal-argument, timeout, cancellation, protocol-error, unsupported-provider, payload-redaction, and operation-correlation tests using local MCP fixtures.

## 9. Commands, Services, Adapters, and UI

- [x] 9.1 Add Rust commands and mappers for observability settings, paginated execution-run listing, run detail, timeline detail, and provider/MCP observation capability.
- [x] 9.2 Add the frontend `ExecutionObservabilityService`, typed models, Tauri adapter, runtime factory, and adapter conformance tests with `invoke()` confined to the Tauri-specific adapter.
- [x] 9.3 Add deterministic Web/mock settings, run summaries, timelines, pagination, and capability results that preserve contract parity without claiming native storage, process, credential, relay, or export effects.
- [x] 9.4 Add observability settings controls with safe defaults, validation feedback, export-state disclosure, capture warnings, retention controls, and localized English/Chinese copy.
- [x] 9.5 Add a session-scoped execution timeline UI showing topology, stage duration, outcome, fidelity, safe ids, opaque gaps, tool/MCP stages, and links to existing session/operation views.
- [x] 9.6 Add React tests and Playwright coverage for settings defaults/validation, Web/mock parity, successful and failed timelines, incomplete tool observations, opaque MCP gaps, pagination, and narrow-window layout.

## 10. Privacy, Reliability, and Performance Verification

- [x] 10.1 Add end-to-end leak tests proving prompts, outputs, credentials, authorization headers, environment values, private paths, MCP bodies, and tool payloads do not reach metadata-only SQLite, unified logs, stderr fallback, or captured OTLP exports.
- [x] 10.2 Add bounded-content tests for explicitly enabled redacted-content capture and prove raw-content capture remains unavailable.
- [x] 10.3 Add metric tests proving high-cardinality run, trace, span, session, message, operation, process, and tool-call ids are never metric dimensions.
- [x] 10.4 Add load and shutdown tests for concurrent Agent runs, event queue bounds, dropped telemetry accounting, retention maintenance, exporter outage, and bounded flush behavior.
- [x] 10.5 Verify that no implementation introduces feature-local diagnostic log files and that all persistent native diagnostics still use unified logging.

## 11. Final Validation

- [x] 11.1 Run `npm run lint`, `npm run test`, and `npm run build`, and resolve every frontend, contract, unit, and Playwright failure.
- [x] 11.2 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --check`, `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo clippy --manifest-path src-tauri/Cargo.toml`, and resolve every failure or warning.
- [x] 11.3 Run `openspec validate agent-execution-observability --strict` and `openspec validate --specs --strict`, then record the successful implementation verification before archive.

## Implementation Verification

- Verified on 2026-07-23 in the `feature/opentelemetry` worktree.
- Frontend lint completed with zero errors; 214 frontend tests and the production build passed.
- Rust formatting, check, and Clippy with warnings denied passed.
- Full Rust test suite passed with 592 unit tests and 8 architecture tests; one process fixture remained intentionally ignored.
- The change validation and all 55 main specification validations passed in strict mode.
