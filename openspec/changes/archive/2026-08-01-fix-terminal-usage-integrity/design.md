## Context

Interactive Agent terminals run behind `portable-pty` and therefore cannot reuse the managed pipeline's structured output parser. The current implementation periodically reads each CLI's persisted session store, creates an empty streaming assistant message before any usage exists, and updates `usage_records` through the sessions gateway. Three assumptions are unsafe: Gemini JSONL lines are revisions rather than immutable messages, working-directory/mtime lookup is not a provider-session identity, and an atomic stop flag does not prove the poll thread has stopped before the final read.

The collection path is desktop-only. React continues to read summaries through `agentService`; Web/mock behavior is unchanged. `agent_runtime` owns provider file/process adapters, `sessions` owns message and usage persistence, and `execution_observability` owns telemetry contracts.

## Goals / Non-Goals

**Goals:**

- Produce one correct cumulative terminal usage observation per VaneHub session and stable Agent id, including after provider-session resume.
- Match Gemini's own last-write-wins JSONL materialization and exact runtime session identity.
- Create a backing assistant message only after non-zero usage exists, reuse an existing terminal-usage message after restart, and propagate transaction failures.
- Serialize periodic and final ingestion by joining the poll thread.
- Record metadata-only terminal Session/Agent/opaque Tool boundary/Process lifecycle telemetry through existing ports.

**Non-Goals:**

- Parsing ANSI/TUI output into concrete tool or MCP events; the required terminal Tool/MCP boundary remains explicitly opaque.
- Changing the frontend service contract, Tauri command DTOs, usage schema, or Web/mock behavior.
- Mapping Gemini's `tool` token field until its additive semantics are verified.
- Retrofitting or deleting historical empty placeholder messages created by released versions.

## Decisions

1. **Materialize Gemini records by id before aggregation.** Direct message lines replace the prior value for the same id; a `$set.messages` snapshot clears and replaces the materialized map, matching Gemini CLI's loader. Alternatives that sum token-bearing lines or merely deduplicate identical token tuples are rejected because later revisions can legitimately change tokens.

2. **Resolve Gemini by exact provider runtime session id.** The terminal invocation already assigns or resumes a known id. The lookup reads top-level `session-*.jsonl` metadata and accepts only an exact `sessionId` match under the registered project slug. It does not fall back to most-recent mtime, which would prefer wrong data over missing data.

3. **Reuse one terminal-usage message per VaneHub session and Agent without a schema change.** The sessions read port gains a narrow query for an existing `source = 'cli-session-log'` usage record. The runtime loads that id at terminal start; when absent, it creates the backing message lazily only after a non-zero observation. The retained terminal registry already prevents concurrent processes for one VaneHub session, and joining the poll removes the remaining in-process creation race. A generalized non-message usage subject remains a future schema redesign, not required to restore integrity now.

4. **Separate observation from persistence.** Provider readers return `TerminalUsageTotals`; the shared persistence step owns lazy message creation and calls the sessions transaction port. Errors propagate to the caller and unified logging. Cache-only totals count as non-zero.

5. **Own the poll thread handle.** The PTY reader sets the stop flag, joins the poll handle, then performs the final read. A panic is converted to a redacted warning while final ingestion still runs. This preserves terminal availability while establishing a happens-before boundary.

6. **Use existing execution-observability ports.** Bootstrap injects execution identity, settings, and telemetry into the terminal runtime. Each fresh PTY process creates a metadata-only run with a Session root span, an Agent child span, an opaque Tool/MCP boundary child, and a Process Exec child. The opaque boundary represents the known interactive CLI boundary, not fabricated concrete tool calls. The process outcome finishes children then the run. Telemetry failure never changes terminal success. No raw command, path, prompt, or output is attached.

## Risks / Trade-offs

- [Risk] Existing databases can contain more than one historical `cli-session-log` row for a session. → Select the most recently updated matching message deterministically and update only it; preserve older history rather than deleting data in a corrective change.
- [Risk] Gemini changes its unversioned recording format. → Keep permissive parsing, exact fixtures for revisions and snapshots, and degrade to no observation rather than guessing.
- [Risk] Joining waits for an in-progress filesystem read. → Reads are local and bounded to one provider session file/row; poll shutdown does not hold the terminal registry lock.
- [Risk] Terminal TUI does not expose structured tool/MCP lifecycle. → Emit the required Tool/MCP boundary with `opaque` fidelity and do not emit fabricated concrete tool-call children.
- [Risk] A telemetry repository failure occurs during terminal startup or exit. → Ignore telemetry return values for execution outcome and rely on the composite telemetry diagnostic path.

## Migration Plan

- No SQLite schema migration is required.
- Existing terminal usage rows remain readable. On the first post-upgrade refresh, the newest matching row becomes the stable update target.
- Rollback restores the prior collector without data conversion; already-corrected totals remain valid usage rows.

## Open Questions

- A future proposal may generalize `usage_records` from message-only subjects to explicit assistant-response and terminal-session subjects. That is intentionally deferred because it requires a versioned table rebuild and UI accounting decisions.
