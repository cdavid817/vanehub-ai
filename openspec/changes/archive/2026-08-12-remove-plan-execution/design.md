## Context

Plan execution is currently implemented end to end: an activity-bar destination lazy-loads Plan Center; a dedicated frontend service selects Tauri or Web/mock adapters; Tauri commands call a `task_orchestration` bounded context; that context owns SQLite repositories, OnePiece planning and worker execution, guarded validation, recovery, worktree preparation, and diagnostics. Historical database migrations and archived OpenSpec changes have already shipped and must remain immutable.

See `proposal.md` for the product motivation and the delta specs for the behavioral removals.

## Goals / Non-Goals

**Goals:**

- Remove every user-reachable Plan execution surface and live runtime entry point in both desktop and Web/mock builds.
- Remove Plan-only frontend and Rust implementation code instead of leaving an unreachable subsystem.
- Remove Plan-specific published APIs from shared contexts when they have no other consumer.
- Keep compilation, adapter contracts, migrations, logs, tests, documentation checks, and strict OpenSpec validation coherent.
- Preserve upgrade compatibility and user-created Git worktrees without destructive cleanup.

**Non-Goals:**

- Removing chat `permissionMode = "plan"` or CLI-specific Plan Mode flags.
- Removing Loop, GroupChat, scheduled tasks, normal OnePiece sessions, general Git worktrees, or guarded validation used by retained workflows.
- Deleting legacy Plan tables or data from existing SQLite databases.
- Editing immutable archived changes to pretend Plan execution never existed.

## Decisions

### 1. Remove the vertical slice rather than only hiding navigation

The activity-bar destination, lazy feature, Plan components, types, service contract, adapters, Tauri commands, bootstrap assembly, and task-orchestration bounded context will be removed together. Keeping backend code after hiding the entry would preserve command surface and maintenance cost without a supported consumer; keeping frontend adapters would misrepresent runtime capability.

### 2. Preserve historical migrations as an inert compatibility tombstone

The migration named `plan-execution-foundation` and its schema application must remain in the ordered migration chain because deleting or renumbering a shipped migration can break version recognition and fresh-versus-upgraded database equivalence. Its table creation code may remain in a narrowly scoped legacy migration module even though no application repository reads or writes those tables. No `DROP TABLE` migration will be introduced, preserving recoverability and avoiding destructive deletion of prior Plan evidence.

An alternative of deleting the migration and tables was rejected because it rewrites history, loses user data, and can cause schema drift across installation ages.

### 3. Remove Plan-only APIs from shared contexts after proving consumer ownership

Plan-specific worktree preparation, OnePiece planning entry points, orchestration correlation fields, and guarded validation helpers will be searched by symbol. APIs with no retained consumer will be removed. Shared primitives used by Loop, sessions, scheduled tasks, or normal Agent execution will stay even if Plan previously consumed them.

This symbol-by-symbol approach avoids treating folder ownership as proof that a helper is Plan-only.

### 4. Keep current specifications truthful while preserving archives

Main capabilities that exist solely for Plan execution will lose all requirements through removal deltas. Mixed capabilities will lose only their Plan-specific requirements. Archived proposal/design/spec/task artifacts remain unchanged as historical records.

### 5. Remove tests with the code and strengthen absence checks where useful

Plan component, adapter, repository, scheduler, and command tests will be deleted with their implementation. Existing navigation and command-registry tests will be updated to assert the retained destinations and commands. Migration tests will continue to assert the historical migration ordering and idempotent schema so compatibility is explicit rather than accidental.

## Risks / Trade-offs

- [A Plan-only symbol is still referenced from a retained path] → Use repository-wide symbol searches plus TypeScript build, Rust formatting, Clippy, tests, and check to expose residual dependencies.
- [Deleting migration wiring breaks old installations or fresh schema construction] → Retain the historical migration identifier and inert schema application; keep migration tests.
- [Plan Mode is accidentally removed with Plan execution] → Treat `permissionMode`, CLI Plan flags, and native-agent read-only Plan Mode as explicit exclusions and verify their tests still run.
- [Existing Plan worktrees become orphaned] → Do not remove, reset, merge, or mutate them; document that Git tooling remains the manual cleanup path.
- [Dead translations or generated schemas drift] → Remove Plan namespaces and regenerate or validate Tauri schemas through the normal build/check workflow rather than editing generated outputs speculatively.

## Migration Plan

1. Land the spec removal and code removal atomically so released behavior and main specifications agree.
2. Existing databases continue applying the historical Plan migration but receive no new Plan writes.
3. Existing Plan rows, worktrees, and branches remain untouched.
4. Rollback consists of restoring the removed runtime/UI code; retained schema and data allow the older code to read prior records without a reverse data migration.
