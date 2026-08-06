## Context

The `multi-agent-coordination` capability shipped on 2026-07-23 as a complete backend with no user-facing surface. Its footprint today is eleven dedicated Rust modules (four Tauri commands, domain, application, executor, scheduler, repository, schema), four frontend files, four methods on the `AgentService` boundary in both adapters, one main spec, one developer-guide page, and one SQLite table created by migration 27. Roughly ten further Rust modules reference it — bootstrap wiring, the command registry, the shared DTO and mapper, and the application/domain error enums.

Nothing consumes it. The only UI attempt was reverted, so removal is a pure subtraction: there is no caller to migrate and no behavior to preserve. The work is mechanical but wide, and it touches persisted user data, so the sequencing and the table disposition deserve decisions before editing.

## Goals / Non-Goals

**Goals:**
- Remove the capability so no coordination code, command, spec requirement, or table remains.
- Keep the Tauri and Web/mock adapters interface-identical after the four methods are withdrawn.
- Leave every other Agent, session, loop, scheduled-task, and observability behavior untouched.
- Keep migration history honest — no renumbering, no rewriting of what already ran on users' machines.

**Non-Goals:**
- Designing any replacement for multi-Agent collaboration. This change only subtracts.
- Fixing the `providers/output.rs` defect where claude-code `result` events with `is_error: true` parse as successful completions. It affects all CLI execution paths and belongs to its own change.
- Rewriting the archived `2026-07-23-add-multi-agent-coordination` artifacts. Archive records stay immutable; this change supersedes them.

## Decisions

### Drop the table with a new migration rather than deleting migration 27's effect

`apply_migration(conn, version, name, f)` is keyed on an explicit version literal and is idempotent: it consults `schema_migrations` and returns early when that version is recorded. Version numbers are literals in source order, not positional, so removing a call does not renumber its neighbours.

That gives a clean split:

- Keep the migration 27 slot as a documented no-op and delete `coordination_schema.rs`. Existing installs already recorded version 27, so nothing re-runs; fresh installs record it without creating the table.
- Add migration **43** running `DROP TABLE IF EXISTS coordination_runs`. On existing installs this removes the table and its two indexes; on fresh installs it is a harmless no-op.

*Alternative — leave the table orphaned:* rejected. It would outlive its only reader and its schema owner, and the retired "Durable coordination lifecycle" requirement explicitly withdraws the persistence contract.

*Alternative — keep entry 27 pointing at a no-op:* **adopted after implementation.** The original decision was to delete the entry, on the grounds that it preserved a dead function purely to hold a slot. Implementation showed the codebase asserts a dense migration sequence in three places, so deleting 27 would leave a permanent hole that every future migration has to carry. Slot 27 is therefore kept as a documented no-op, and 43 drops what it left behind.

### Number the new migration 43, not the next free 42

Every worktree on a developer machine shares one `ai.vanehub.app` SQLite database, so an unmerged branch's migration can already be recorded there. The concurrently-developed `permissions-core` branch claimed 42 and has run it locally. Because `apply_migration` skips any version already present in `schema_migrations`, reusing 42 would leave this migration permanently skipped on such machines and the table never dropped — the retirement would silently not happen.

Numbering 43 leaves 42 reserved for that branch, which keeps the sequence dense once both land. The cost is that until `permissions-core` merges, this branch's applied set is `1..=41` plus `43`; the fixture tests encode that gap explicitly with a comment rather than pretending it is contiguous.

### Remove frontend and native sides independently, each outside-in

The two sides couple only through `invoke()` command-name strings, so neither blocks the other. Within each side, removal proceeds from the outermost consumer inward so the compiler surfaces one layer of breakage at a time:

- **Frontend:** the four `AgentService` methods and both adapter implementations, then `coordination-runtime.ts` and `types/coordination.ts` with their tests.
- **Native:** the command modules and their registry entries, then the `AgentRuntimeApi` methods, then application, then domain, then infrastructure (executor, scheduler, repository), then the schema and its migration entry.

The coordination variants in the shared DTO, mapper, and the application/domain/command error enums come out last on the native side, because `cargo check` will point at every exhaustive `match` that still names them.

### Trim observability code to match the narrowed spec

The modified `agent-execution-observability` requirements drop coordination nodes, failover attempts, and the `candidate role` metric dimension. The corresponding telemetry emission and metric dimensions are removed with them, so the code and spec retire together rather than leaving instrumentation for events that can no longer occur.

## Risks / Trade-offs

- **Dropping `coordination_runs` is irreversible for existing installs** → Accepted deliberately. The rows record executions of a feature that never had a user-facing surface, so there is no history a user could recognise or miss. No export path is provided; the proposal states this plainly.
- **Removing error-enum variants can break exhaustive matches in unrelated modules** → `cargo check` enumerates every site. No behavior change is intended at any of them; each is a mechanical arm deletion.
- **`composite_process_gateway.rs` and `runtime_support.rs` are shared with non-coordination execution paths** → Remove only the coordination-specific members from them rather than the files, and rely on the full native test suite to catch overreach.
- **Build verification can be invalidated by pnpm contamination** → This worktree has repeatedly acquired a pnpm-shaped `node_modules`, which silently breaks the katex chunk-split rule and fails `npm run build` on an unrelated chunk. Run `npm ci` and confirm `node_modules/.pnpm` is absent before trusting any verification result.
- **Shared local database lets one worktree's migration number collide with another's** → Discovered during implementation, not anticipated. Mitigated here by numbering 43 and by dropping the stale table from the local database by hand, since the recorded 42 means the migration cannot backfill it. Any future migration in this repo should check `schema_migrations` on a real machine, not just the highest literal in source.
- **Wide diff across two languages raises review cost** → Mitigated by the outside-in ordering, which keeps each commit's breakage confined to one layer.

## Migration Plan

1. Remove the frontend surface (methods, adapters, runtime, types, tests) and confirm `npm run lint`, `npm run test`, and `npm run build`.
2. Remove the native surface outside-in and confirm `cargo check` and `cargo clippy` after each layer.
3. Add migration 43 dropping the table; delete `coordination_schema.rs` and reduce slot 27 to a no-op.
4. Delete `openspec/specs/multi-agent-coordination/spec.md`, apply the observability spec delta, and remove the developer-guide page and its `SUMMARY.md` entry.
5. Verify with `openspec validate --specs --strict`, the full frontend and native suites, and the Playwright suite pinned to a self-started dev server.

**Rollback:** revert the change to restore all code, specs, and docs. The dropped table cannot be restored — any install that has run migration 43 has lost its coordination rows permanently.

## Open Questions

None. The table disposition, the migration numbering, and the developer-guide removal are all decided above.
