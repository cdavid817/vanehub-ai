## Context

The existing Skills settings page is a static React demo backed by `settings-demo-data`. It does not use the frontend service boundary, does not persist Skill state, and cannot reconcile filesystem state for Agent-specific skill directories.

This change introduces a real Skill management subsystem. It spans the settings UI, the frontend service interface and adapters, Rust Tauri commands, SQLite state, and local filesystem operations. The design is informed by the Skill management patterns in `D:\cdavid\Documents\code\clowder-ai`, especially `SKILL.md` metadata parsing, separated query/mount/drift logic, and drift sync reporting. VaneHub will not copy Clowder's per-provider or all-project cascade model; VaneHub's mount carrier is the registered Agent and the workspace directory is the scope boundary.

## Goals / Non-Goals

**Goals:**

- Manage Skills in two isolated scopes: global and workspace.
- Store global Skills under a fixed user-home VaneHub directory.
- Store workspace Skills inside the selected project directory.
- Use registered Agents as mount carriers with editable mount paths.
- Mount Skills by symlink or directory link.
- Seed six built-in Skills idempotently and support restore after deletion.
- Standardize `SKILL.md` frontmatter and make Skill `id` immutable after creation.
- Detect drift across SQLite records, source `SKILL.md`, and Agent mount paths.
- Sync drift with backup-and-overwrite conflict handling.
- Replace `skills-page.tsx` with a service-driven UI composed from seven reusable child components.
- Keep browser/Web runtime functional through a Web/mock adapter.

**Non-Goals:**

- Do not implement remote Skill marketplaces.
- Do not synchronize workspace Skills across multiple projects.
- Do not allow editing a Skill id after creation.
- Do not move SQLite or filesystem mutation logic into React.
- Do not introduce Redux, Zustand, MobX, CSS Modules, styled-components, or a new UI component library.

## Decisions

### Decision: Model Skills as scope-bound source directories

Global Skills live under a fixed user-home directory, such as `~/.vanehub/skills/<skill-id>/SKILL.md`. Workspace Skills live under `<workspace>/.vanehub/skills/<skill-id>/SKILL.md`.

Rationale: This keeps global and workspace ownership separate and makes the workspace directory the exact boundary requested by the product design.

Alternative considered: Store all Skills under the app data directory and only mount them into workspaces. That would make workspace Skills less inspectable from the project and would weaken the "directory is boundary" rule.

### Decision: Treat `SKILL.md` as the metadata and content carrier

Each Skill directory must contain `SKILL.md` with frontmatter:

```yaml
---
id: tdd-discipline
name: TDD 开发纪律助手
description: 引导开发过程遵循测试先行、红绿重构和回归验证纪律。
category: development
version: 1.0.0
triggers:
  - TDD
  - 测试先行
---
```

The backend parses metadata, stores a snapshot and content hash in SQLite, and rejects id changes after creation.

Rationale: A single standard file makes Skills portable and enables deterministic drift checks.

Alternative considered: Separate JSON metadata and Markdown body. That would add another file format and create two truth surfaces.

### Decision: Seed built-in Skills with deletion markers

The built-in set is fixed:

- `tdd-discipline`
- `code-review`
- `code-security-scan`
- `api-doc-generation`
- `unit-test-generation`
- `readme-generation`

Initialization is idempotent, but deleting a built-in Skill records a deleted marker so startup seeding does not silently recreate it. A restore action clears the marker, rewrites the standard `SKILL.md`, and recreates the registry entry.

Rationale: This satisfies both idempotent initialization and user-controlled deletion.

Alternative considered: Never allow built-in deletion. The user explicitly confirmed deletion and restore should both exist.

### Decision: Use registered Agent ids for mount paths

Mount paths are keyed by stable `AgentRegistryEntry.id`, not display names or provider labels. Defaults are:

- `claude-code`: `.claude/skills`
- `codex-cli`: `.codex/skills`
- `gemini-cli`: `.gemini/skills`
- `opencode`: `.opencode/skills`

Users may edit each Agent mount path. When a path changes, the backend immediately migrates existing managed links for that Agent and returns a migration report.

Rationale: Agent ids already form the stable registry boundary in VaneHub. Immediate migration prevents UI state from diverging from filesystem state.

Alternative considered: Store provider-level mount paths. That would not work for multiple Agents from the same provider or future custom Agent registrations.

### Decision: Link-based mount with backup-and-overwrite conflicts

Mounting creates a symlink or directory link from the Agent mount directory to the Skill source directory. On Windows, the implementation should try directory symlink first, then junction where appropriate. If a same-name file, directory, or foreign link occupies a target path, sync backs it up under `.vanehub/backups/skills/<timestamp>/` before replacing it.

Rationale: The user selected soft links/directory links and overwrite conflict behavior. Backing up before overwrite keeps the product behavior decisive without making data loss silent.

Alternative considered: Skip conflicts and require manual repair. That is safer by default but conflicts with the requested overwrite behavior.

### Decision: Keep Skill APIs inside `AgentService`

React components call new methods on `AgentService`. `tauri-agent-client.ts` maps those methods to Tauri commands, while `web-agent-client.ts` provides in-memory mock behavior.

Rationale: This follows the existing project architecture: components must not call Tauri `invoke()` directly, and both runtimes must expose the same service shape.

Alternative considered: Create a separate Skill service. That could be clean later, but the current settings pages already depend on the agent service boundary for registered Agents and runtime-specific behavior.

### Decision: Rebuild the Skills settings page as a container plus seven children

`skills-page.tsx` becomes a state orchestration container. It loads Agents, mount paths, Skills, stats, drift reports, and migration/sync results through the service. New reusable child components:

- `SkillStatsCards`
- `SkillAgentMountPathsPanel`
- `SkillScopeTabs`
- `SkillFilterToolbar`
- `SkillCardList`
- `SkillDialogs`
- `SkillDriftBanner`

Rationale: The current page is static and too flat. This split maps directly to product modules and keeps future iteration contained.

Alternative considered: One large page component. That would exceed the project's component size guidance and make dialog/list/drift behavior harder to test.

## Risks / Trade-offs

- [Risk] Windows symlink creation may fail without privileges. -> Mitigation: fall back to directory junction and return structured mount errors if both strategies fail.
- [Risk] Backup-and-overwrite may surprise users who had custom files in Agent skill directories. -> Mitigation: sync reports must include overwritten and backup paths, and drift detection must preview conflicts before sync.
- [Risk] Editable Agent mount paths can break existing links if migration partially fails. -> Mitigation: migrate per Agent with a report containing migrated, removed, overwritten, backed up, and failed entries; preserve old links when a target migration fails.
- [Risk] Built-in deletion can conflict with seed initialization. -> Mitigation: persist deleted built-in markers and only restore through explicit user action.
- [Risk] Web/mock behavior cannot perform real filesystem operations. -> Mitigation: mock adapter returns deterministic in-memory Skills, mount paths, and drift states while marking local filesystem operations as simulated.

## Migration Plan

1. Add SQLite migration for Skill registry tables, Agent mount path settings, bindings, drift snapshots, and built-in deletion markers.
2. Add Rust Skill domain modules for metadata parsing, source path resolution, built-in seeding, mount operations, drift detection, and sync.
3. Add Tauri commands and register them in the command handler.
4. Extend `AgentService` and both frontend adapters.
5. Replace the static Skills page with service-backed UI components.
6. Add tests for metadata parsing, seed idempotency, mount path migration, drift detection, Web/mock adapter behavior, and Playwright UI flows.

Rollback strategy: because this is additive, rollback removes the UI entry points and commands while leaving Skill files and SQLite records inert. Managed symlinks should not be deleted automatically during rollback.

## Open Questions

- The exact fixed home directory name should be finalized during implementation. The design assumes `~/.vanehub/skills`.
- The built-in Skill body text can be implemented from product-approved starter prompts if no separate content source is provided.
