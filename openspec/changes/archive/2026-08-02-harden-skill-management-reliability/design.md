## Context

Skill management spans React Query state, the `AgentService` runtime boundary, Tauri commands, a Rust application service, SQLite, managed source directories, CLI mount links, and API-agent system-prompt assembly. The current implementation keeps these layers separated, but several invariants are only implied: every registered Agent is treated as a mount carrier, workspace database keys are not canonicalized with filesystem paths, edits replace whole directories, filesystem transactions are not serialized, and API bindings are fetched per Skill and injected without workspace or size bounds.

The change must preserve the existing React/Tauri/Web separation, remain compatible with SQLite and local files, and avoid introducing a new state library or runtime dependency. Skill mutations are infrequent compared with reads and API generations, so correctness and deterministic recovery take priority over maximizing mutation concurrency.

## Goals / Non-Goals

**Goals:**

- Establish explicit invariants for Agent type, workspace identity, source/mount separation, mutation serialization, restore eligibility, and prompt selection.
- Preserve imported Skill assets and reject implicit source overwrites.
- Make the desktop and Web/mock adapters behaviorally equivalent for validation, content, bindings, drift, and deletion.
- Reduce the settings-page load path from per-Skill IPC/SQL work to a fixed number of batch operations.
- Bound import and API prompt costs and report deterministic omissions or failures.
- Clean up existing orphan Skill-to-Agent state without deleting user Skill source directories.

**Non-Goals:**

- Remote Skill marketplaces, network synchronization, vector retrieval, or model-selected progressive Skill loading.
- Per-Skill user-configurable prompt order or provider-specific tokenizers.
- Automatically deleting an old managed mount after a partial migration when ownership cannot be proven.
- Changing the standard `SKILL.md` metadata fields or introducing a second Skill document format.

## Decisions

### 1. Model CLI and API binding targets as distinct capabilities

Repository Agent queries will expose launch kind. Mount-path configuration and CLI bindings accept only CLI-capable Agents; API prompt bindings accept only API Agents. The Skills page receives separate `cliAgents` and `apiAgents`, and no UI branch infers capability from display names.

The native migration removes orphan bindings and removes CLI/mount-path rows for API Agents. API Agent deletion explicitly clears CLI bindings, API bindings, and mount-path configuration in the same SQLite transaction.

Alternative considered: filter only in React. Rejected because stale clients, Web mode, tests, or direct command calls could still create invalid state.

### 2. Canonical workspace identity is established at the native boundary

Workspace paths are canonicalized before constructing `SkillLocation`; the canonical display-safe path becomes the SQLite key and filesystem root. Global scope continues to normalize to an empty storage key. On Windows, canonical comparison is case-insensitive.

The Web adapter applies a deterministic lexical normalization suitable for mock paths. Existing non-canonical native rows are not silently merged when both aliases contain data; they are surfaced as a migration/drift conflict so no source is overwritten.

Alternative considered: canonicalize only in the filesystem adapter. Rejected because it permits multiple database identities to address the same directory.

### 3. Serialize native Skill mutations and make edits narrow

A shared `SkillMutationCoordinator` serializes Skill filesystem/database mutations. A global mutex is deliberately chosen for the first hardening pass: mutations are user-driven and rare, while a keyed lock would require ordered multi-key acquisition for drift sync and mount migration.

`updateSkill` changes metadata and `SKILL.md` body only. It no longer accepts or rewrites enabled state or bindings. The request carries the previewed `expectedContentHash`; the application service rejects the update if the registry or live document hash changed. CLI binding UI uses granular bind/unbind operations so rapid independent checkbox changes cannot overwrite one another with stale full sets.

Alternative considered: add a persisted revision column to every Skill mutation. Rejected for this pass because narrow commands plus content-hash conflict detection cover the observed races with less migration surface.

### 4. Preserve sources and reserve managed namespaces

Edits atomically replace only `SKILL.md`, leaving scripts, templates, and other imported files intact. Create, import, seed, and restore fail when a target source directory already exists without the exact expected registry/tombstone state. Restore requires a matching deleted-built-in tombstone.

Mount paths under `.vanehub` are rejected, and the filesystem adapter performs a final canonical disjointness check between source and target before moving or linking anything. Import rejects a source equal to, containing, or contained by the destination and enforces 512 files, depth 16, 16 MiB aggregate bytes, and a 256 KiB `SKILL.md` limit.

Transient filesystem actions remain reversible during a process lifetime. Stale `.vanehub-transaction-*` artifacts encountered on a managed path are reported as drift rather than silently adopted or removed; users retain a recovery path without speculative deletion.

### 5. Intentional deletion and drift repair remain separate

Deleted built-in tombstones are queried through a restore-candidate operation, not emitted as drift. Drift synchronization validates refreshed metadata identity before persistence. Missing or unregistered sources that cannot be repaired unambiguously remain reported and are not described as successfully synchronized.

Alternative considered: keep deleted built-ins as drift and let one-click sync restore them. Rejected because deletion is an intentional supported state, not corruption.

### 6. Load a batch Skill overview

`AgentService.getSkillOverview(scope)` returns Skills, stats, drift, mount paths, separated Agent lists, API binding ids, and restore candidates. Native repository loading uses fixed batch queries and merges bindings in memory. Supporting indexes cover scope/workspace listing and Agent-oriented binding lookup.

The settings page uses one overview query per scope and targeted cache updates/invalidations after mutations. Loading, error, stale-write, and partial migration states are explicit; null data is never rendered as a healthy drift result.

Alternative considered: preserve the per-Skill `useQueries` calls and add caching. Rejected because initial IPC and SQLite work remains O(number of Skills).

### 7. Scope and bound API prompt injection

`AgentSkillPort::bound_skill_prompts` receives the current canonical workspace. It selects enabled global Skills plus enabled Workspace Skills whose canonical key matches that workspace. Individual unreadable sources are logged and skipped without suppressing healthy Skills.

Prompts remain ordered deterministically by scope, workspace, and Skill id. A Skill contributes at most 8,000 characters and all Skills together at most 16,000 characters. An oversized Skill is skipped rather than partially truncating instructions; omissions emit a warning through the unified logging boundary. The assembled system prompt remains outside compactable turns.

Alternative considered: tokenize per provider. Rejected because provider-specific tokenizers would add dependencies and still require a conservative common fallback.

### 8. Give the Web adapter a real in-memory document model

Web/mock stores `{skill, document}` records, uses the same externally visible validation rules and limits, cleans both binding types on deletion, scopes restore candidates globally, and filters mock generation by session workspace. Filesystem-specific mount and drift results remain simulated but deterministic.

## Risks / Trade-offs

- [Risk] Global mutation serialization can delay a second mutation during a large import. → Mitigation: strict import limits bound the critical section; reads and API generations do not acquire the mutation lock.
- [Risk] Canonicalizing legacy workspace keys can reveal aliases that previously appeared independent. → Mitigation: report conflicts and require explicit reconciliation instead of auto-merging source content.
- [Risk] A 16,000-character prompt budget may omit a previously injected large Skill. → Mitigation: deterministic omission, visible warning logs, and UI metadata make the limit observable.
- [Risk] Batch overview is a wider DTO. → Mitigation: keep operations behind `AgentService`, add contract-conformance tests, and retain focused mutation commands.
- [Risk] Existing invalid API-Agent CLI bindings may have live links. → Mitigation: remove database ownership records during migration but only remove filesystem links when the target is proven to point at the managed source.

## Migration Plan

1. Ensure the reliability migration creates the API-Agent Skill binding table before cleanup so databases that already recorded the historical Skill migration can upgrade safely, then add indexes and clean up orphan bindings, API-Agent CLI bindings, and API-Agent mount-path rows while preserving all Skill source directories.
2. Add canonical location, Agent-kind validation, reserved-path validation, mutation serialization, narrow edit semantics, and restore/drift safety.
3. Add batch repository/service/command/adapter DTOs while retaining old read commands temporarily for compatibility.
4. Switch the settings UI to the batch overview and granular CLI binding mutations, then remove active use of per-Skill API binding queries.
5. Update API runtime selection and prompt budgets.
6. Align Web/mock state and validations.
7. Run migration fixtures, Rust unit/integration tests, frontend tests, build, Clippy, and strict OpenSpec validation.

Rollback keeps the additive indexes and cleaned orphan rows. Application rollback can continue reading the unchanged core Skill tables; no managed source directory is removed by the migration.

## Open Questions

None. Limits and conflict behavior are fixed by this design so implementation can proceed without additional product decisions.
