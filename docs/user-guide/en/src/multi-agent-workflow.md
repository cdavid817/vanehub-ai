# Multi-Agent coding workflow preview

**Status: Preview — runtime/service contract exists; creation UI is unavailable.**

The intended workflow decomposes one coding goal into a dependency graph:

```text
plan
  +--> frontend (primary: codex-cli, fallback: claude-code) --+
  +--> native   (primary: claude-code, fallback: codex-cli) --+--> test --> review
```

The `frontend` and `native` nodes are independently ready. `test` waits for both outputs, and `review` waits for testing.

For each node, the plan records:

- a stable node id;
- a primary stable Agent id;
- ordered fallback Agent ids;
- an instruction;
- prerequisite node ids.

Successful prerequisite output is passed to dependents with source and actual-Agent provenance. Retryable execution failures advance through fallbacks in order. Validation, policy, cancellation, persistence, and context-limit failures do not trigger fallbacks.

Cancellation stops the active attempt and prevents new attempts. Desktop state is durable in SQLite; Web/mock state is simulated.

There are intentionally no click-by-click creation steps or UI screenshots here. The normal create-session UI still marks Multi Agent as unavailable. This chapter can become a delivered workflow only after user-visible controls and a Playwright path can create, observe, and complete or cancel a run.
