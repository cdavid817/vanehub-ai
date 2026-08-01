## 1. Gemini provider session integrity

- [x] 1.1 Materialize Gemini JSONL direct revisions and `$set.messages` snapshots by message id before token aggregation.
- [x] 1.2 Resolve Gemini chat files by exact runtime session id and validate project slug/file boundaries.
- [x] 1.3 Add regression tests for duplicate tool-call revisions, snapshots, exact session selection, and missing/ambiguous ids.

## 2. Stable and honest usage persistence

- [x] 2.1 Add a narrow sessions port/API/repository query for the existing terminal usage message of a VaneHub session and stable Agent.
- [x] 2.2 Create the usage backing message lazily on the first non-zero observation and reuse it across polls and terminal restarts.
- [x] 2.3 Propagate transaction failures and persist cache-only reported observations.
- [x] 2.4 Add tests for restart-safe reuse, no empty streaming placeholder, cache-only usage, and persistence failure propagation.

## 3. Terminal lifecycle and observability

- [x] 3.1 Retain and join the periodic poll thread before final ingestion, with redacted diagnostics for worker failure.
- [x] 3.2 Inject execution identity/settings/telemetry into the PTY runtime and emit metadata-only Session, Agent, opaque Tool/MCP boundary, and Process Exec lifecycle spans.
- [x] 3.3 Add deterministic tests for poll shutdown ordering, telemetry topology/outcomes, and PTY cleanup behavior.

## 4. Verification

- [x] 4.1 Run Rust formatting, unit tests, architecture tests, check, and clippy with warnings denied.
- [x] 4.2 Run frontend lint, tests, and production build to verify unchanged service/adaptor behavior.
- [x] 4.3 Run `openspec validate fix-terminal-usage-integrity --strict` and `openspec validate --specs --strict`.
