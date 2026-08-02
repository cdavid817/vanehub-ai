## Why

Skill management currently permits Agent-type and workspace-boundary leaks, unsafe source-directory replacement, stale concurrent writes, and unbounded per-generation prompt injection. These defects can lose imported Skill assets, contaminate API sessions across projects, leave orphan bindings, or make a Skill unreadable after drift synchronization, so the subsystem must be hardened before it scales beyond the small built-in catalog.

## What Changes

- Separate CLI mount bindings from API-agent prompt bindings at the UI, service, repository, and lifecycle boundaries, and reject mount paths that overlap VaneHub-managed source storage.
- Canonicalize workspace identity before persistence and restrict API prompt injection to global Skills plus Skills from the active workspace.
- Preserve non-`SKILL.md` assets during edits, reject implicit source replacement, require a deleted-built-in tombstone for restore, and keep intentional deletion out of drift repair.
- Validate drift refreshes before persistence, serialize Skill mutations, reject stale updates by content hash, and recover or surface incomplete filesystem operations safely.
- Replace per-Skill binding requests and repository N+1 loading with a batch Skill overview and supporting SQLite indexes.
- Bound Skill prompt injection and external imports with deterministic limits and diagnostic reporting.
- Make the Web/mock adapter enforce the same validation, content lifecycle, binding cleanup, scope, and error semantics as the desktop adapter.
- Add explicit loading/error states and interaction-safe mutation behavior to the Skills settings page.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `skill-management`: Strengthen scope identity, source safety, binding type rules, mutation consistency, drift/restore semantics, batch loading, import limits, and Web parity.
- `settings-skill-management-ui`: Separate CLI/API targets, preserve editable content, prevent stale concurrent mutations, and expose accurate loading, error, conflict, and recovery states.
- `agent-skill-injection`: Scope Workspace Skills to the active workspace, enforce a deterministic prompt budget, and retain healthy Skills when one bound source fails.
- `agent-lifecycle-management`: Remove all CLI/API Skill bindings and mount-path state when an API Agent is deleted.

## Impact

- Desktop runtime: Rust Skill domain/application services, SQLite schema and migrations, filesystem transaction handling, Agent deletion, and API generation prompt resolution.
- Web runtime: in-memory Skill document storage, validation, drift, binding, lifecycle cleanup, and mock generation behavior.
- Frontend boundary: `AgentService`, Tauri/Web adapters, Skill DTOs, React Query loading strategy, and Skills settings components.
- Existing data: additive indexes plus cleanup/canonicalization handling for orphan or semantically invalid bindings; no user Skill source directory may be silently overwritten during migration.
- Architecture: React continues to depend only on the service boundary, and runtime-specific behavior remains behind Tauri and Web adapters with equivalent contracts.
