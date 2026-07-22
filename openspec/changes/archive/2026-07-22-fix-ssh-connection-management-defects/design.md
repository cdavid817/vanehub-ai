## Context

SSH connection management spans React forms, frontend service adapters, Rust application logic, native secure storage, and SQLite. The initial implementation passes clean-database tests but mutates migration 20 after it has already shipped, so an installed database at version 23 skips the new port-column statements when it applies migration 24. Failure handling also updates secure storage and SQLite in separate steps without complete compensation, while two UI paths do not reflect backend validation or persisted failure state.

## Goals / Non-Goals

**Goals:**

- Make migration 24 self-sufficient for both clean and already-migrated databases.
- Keep SQLite profile metadata and native password storage coherent when mutations fail.
- Ensure UI submission and cache state reflect the persisted SSH connection outcome.
- Add regression tests that exercise upgrade and failure paths rather than only happy paths.

**Non-Goals:**

- Add SSH authentication or remote command execution to the TCP connection probe.
- Change Tauri command payloads or expose secrets to React.
- Introduce cross-resource transactions between SQLite and the operating-system credential store.

## Decisions

1. Migration 24 will call a local schema function that first applies the SSH profile schema and then idempotently adds the two remote workspace port columns. The original migration 20 remains capable of creating a fresh remote workspace schema, while upgrade correctness no longer depends on rerunning it.

2. Migration tests will construct a database through migration 23, remove migration records 24 and its SSH table when necessary, then apply the current migration and assert both new columns and preserved rows. This models an installed pre-feature database instead of inserting data after migrating to the latest version.

3. Credential mutation will track whether the current operation created, replaced, or removed a secret. If the SQLite update fails, it will restore the previous secret or delete a newly-created secret. Deletion will retain enough profile state to restore the database record when secure-storage deletion fails, avoiding an unreachable credential.

4. React Query invalidation will run in the connection-test mutation's settled path so both successful and failed persisted status changes become visible.

5. Create-session eligibility will validate the derived save-as-connection input when that option is enabled. The same pure validator will be used before submission, with localized actionable feedback instead of collapsing profile validation failures into a generic command error.

## Risks / Trade-offs

- [Secure storage and SQLite cannot share a native transaction] -> Use explicit compensation and focused failure tests; return an error if compensation itself cannot restore coherence.
- [A migration fixture assembled through current migration functions can accidentally include newer schema] -> Explicitly assert the simulated pre-upgrade columns are absent before applying migration 24.
- [Stricter save-as-connection validation can disable session creation unexpectedly] -> Apply it only while the save option is checked; manual temporary remote sessions retain existing validation.

## Migration Plan

Migration 24 remains the latest migration number because the feature has not been committed or released. Its implementation becomes idempotent and handles clean databases plus databases whose schema migration table already contains versions 1 through 23. No rollback data transformation is required.

## Open Questions

None.
