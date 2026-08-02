## 1. Native identity, schema, and source safety

- [x] 1.1 Add Skill schema indexes and migration cleanup for orphan bindings, API-Agent CLI bindings, and API-Agent mount-path rows without deleting Skill sources.
- [x] 1.2 Canonicalize Workspace Skill locations at the native command boundary and detect ambiguous legacy aliases safely.
- [x] 1.3 Restrict mount paths to CLI-capable Agents and reject `.vanehub` or source-overlapping targets in both domain and filesystem layers.
- [x] 1.4 Add a shared native Skill mutation coordinator and narrow Skill edits to conflict-aware atomic `SKILL.md` replacement that preserves other assets.
- [x] 1.5 Enforce create/import/seed/restore source collision rules and bounded, non-overlapping recursive imports.
- [x] 1.6 Separate deleted-built-in restore candidates from drift and validate refreshed metadata identity before synchronization persistence.

## 2. Binding and lifecycle correctness

- [x] 2.1 Add granular CLI Skill bind/unbind application operations and Tauri commands while retaining compatibility for existing bulk callers.
- [x] 2.2 Validate CLI versus API Agent kind for every binding operation and exclude API Agents from mount-path configuration queries.
- [x] 2.3 Delete all API/legacy CLI Skill bindings and mount-path state atomically when an API Agent deletion succeeds.
- [x] 2.4 Add native tests for Agent-kind rejection, rapid independent bindings, restore eligibility, drift identity safety, and deletion cleanup.

## 3. Batch Skill overview and query performance

- [x] 3.1 Add batch repository reads for Skill records, CLI/API bindings, compatible Agents, restore candidates, and drift data without per-Skill queries.
- [x] 3.2 Add `SkillOverview` models, application/API method, Tauri command, TypeScript contracts, and both runtime adapter methods.
- [x] 3.3 Add query-count/index coverage proving overview loading remains O(1) in statement count as Skill count grows.

## 4. Workspace-aware bounded API injection

- [x] 4.1 Pass the active canonical workspace through `AgentSkillPort` and select only applicable global and matching Workspace Skill bindings.
- [x] 4.2 Skip and log individual unreadable Skill sources while retaining healthy bound Skills.
- [x] 4.3 Enforce deterministic 8,000-character per-Skill and 16,000-character aggregate prompt budgets with unified warning logs.
- [x] 4.4 Add runtime tests for workspace isolation, deterministic order, partial read failure, and both prompt budgets.

## 5. Web adapter and Skills settings UI

- [x] 5.1 Store full in-memory Web Skill documents and align native validation, restore, drift, import, and binding cleanup semantics.
- [x] 5.2 Refactor the Skills page to one overview query with separated CLI/API Agent controls and explicit loading/error states.
- [x] 5.3 Load current Skill body and content hash for editing, preserve dialogs on conflicts, list only restore candidates, and confirm destructive deletion.
- [x] 5.4 Replace stale bulk checkbox updates with guarded granular mutations, fix combined search behavior, and target cache updates narrowly.
- [x] 5.5 Add frontend and Web adapter tests for content preservation, scope parity, cleanup, Agent separation, errors, search, conflicts, and rapid bindings.

## 6. Verification

- [x] 6.1 Run `npm run lint`, `npm run test`, and `npm run build` and resolve all failures.
- [x] 6.2 Run `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo clippy --manifest-path src-tauri/Cargo.toml` and resolve all failures.
- [x] 6.3 Run `openspec validate harden-skill-management-reliability --strict` and `openspec validate --specs --strict`, then record implementation verification results.

## 7. Verification remediation

- [x] 7.1 Prevent loading or failed Skill overview requests from rendering healthy empty-state content and add regression coverage.
- [x] 7.2 Keep stale edit conflicts inside the edit dialog, provide an explicit reload action, and surface edit-preview failures.
- [x] 7.3 Align Web Skill identity, scope, and import validation outcomes with the native boundary and add parity tests.
- [x] 7.4 Instrument the batch overview repository test to prove SQL statement count remains constant as Skill count grows.
- [x] 7.5 Add native import tests for file-count, aggregate-size, and source/destination overlap limits, then rerun all required validation.
- [x] 7.6 Apply repository rustfmt output and rerun native quality gates after the Linux CI formatting failure.

## 8. Existing-database migration remediation

- [x] 8.1 Create `skill_api_agent_bindings` idempotently at the start of migration 37 before its cleanup and index statements.
- [x] 8.2 Add an upgrade regression test for a database with migrations 1-36 recorded and no `skill_api_agent_bindings` table.
- [x] 8.3 Rerun the complete frontend, Rust, and strict OpenSpec validation suite and record the results.
