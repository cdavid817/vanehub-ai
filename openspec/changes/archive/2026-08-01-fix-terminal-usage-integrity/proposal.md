## Why

Interactive terminal usage tracking can silently overcount Gemini CLI tool-call turns, attach usage to the wrong provider session, duplicate historical totals after resume, and report successful persistence when SQLite writes fail. The same lifecycle also creates empty streaming assistant placeholders and allows the periodic poll to race the final read, so the shipped usage statistics are not yet a trustworthy read model.

## What Changes

- Materialize Gemini CLI's append-only JSONL by message id with last-write-wins semantics, including `$set.messages` snapshots, before aggregating tokens.
- Resolve Gemini chat files by the provider runtime session id already assigned to the terminal instead of guessing from working directory and modification time.
- Make terminal usage persistence lazy and stable across terminal restarts, propagate persistence failures, preserve cache-only observations, and avoid empty streaming placeholder messages.
- Own and join the periodic usage-poll thread before the exit-time read so older work cannot overwrite the final observation.
- Correlate interactive Agent terminal and PTY process lifecycle telemetry using the existing execution-observability boundary, without capturing terminal content or inventing unobservable tool/MCP details.
- Add regression tests for duplicate JSONL revisions, exact session selection, resume-safe upsert, persistence failures, cache-only observations, and poll shutdown ordering.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `usage-statistics`: Require interactive terminal usage to materialize provider-native revisions, bind to the exact provider session, update one stable observation across resume, and surface persistence failures.
- `agent-terminal-runtime`: Require usage polling to stop and join before final ingestion and forbid terminal usage tracking from leaving empty streaming chat messages.
- `agent-execution-observability`: Extend correlated Agent/process lifecycle telemetry to interactive embedded Agent terminals while retaining metadata-only privacy and fidelity rules.

## Impact

- Desktop/Tauri runtime only for collection and telemetry; Web/mock behavior and frontend service contracts remain unchanged.
- Rust changes are scoped to `agent_runtime` terminal/session-capture infrastructure, its application ports, the published `sessions` API and SQLite repository queries, plus bootstrap telemetry assembly.
- No React component gains a direct Tauri dependency and no Tauri command signature changes.
- The existing `usage_records` schema is reused; no SQLite migration is required for this correction.
- No new dependency is introduced.
