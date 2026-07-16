## Why

The current Skills settings page is a static demonstration surface and cannot manage real reusable Agent skills. VaneHub needs a service-driven Skill management system that supports global and workspace scopes, Agent-specific mount paths, drift detection, and built-in starter skills.

## What Changes

- Add first-class Skill management with separate global and workspace scopes.
- Use registered Agents as Skill mount carriers, with one editable mount path per Agent and immediate migration when paths change.
- Standardize Skill definition on `SKILL.md` with fixed frontmatter schema and immutable `id` after creation.
- Seed six built-in Skills idempotently and support disabling, deleting, and restoring them.
- Store global Skills under a fixed user-home VaneHub directory and workspace Skills under the selected project directory.
- Import external Skills by copying them into the managed Skill source directory for the selected scope.
- Mount Skills into Agent-specific paths through symlinks or directory links.
- Detect configuration drift between SQLite records, source `SKILL.md` files, and Agent mount paths, then provide sync with backup-and-overwrite conflict handling.
- Replace `skills-page.tsx` with a dynamic service-backed Settings page and seven reusable child components for stats, mount paths, scope selection, filtering, cards, dialogs, and drift warnings.
- Support workspace directory selection through the desktop runtime while keeping the Web/mock adapter usable.

## Capabilities

### New Capabilities

- `skill-management`: Manage global and workspace Skills, Agent mount bindings, built-in Skill seeds, `SKILL.md` metadata, drift detection, synchronization, import, and restore behavior.

### Modified Capabilities

- `settings-center-ui`: The Skills settings page becomes a service-driven management UI with scope tabs, Agent mount path editing, search/filtering, Skill cards, dialogs, drift banner, and summary statistics.

## Impact

- Affects both desktop runtime and Web runtime.
- Extends `src/services/agent-service.ts` with Skill management APIs and implements matching Tauri and Web/mock adapters.
- Adds Tauri commands and SQLite-backed Skill registry/mount state under `src-tauri/`.
- Reworks `src/settings/pages/skills-page.tsx` and adds seven reusable Skill settings components.
- Requires filesystem operations in the Rust layer for home/workspace Skill directories, symlink or directory-link mounting, backup-and-overwrite sync, and directory picker integration.
- Preserves frontend/backend isolation: React components must call the service interface only, and Tauri `invoke()` remains limited to `tauri-agent-client.ts`.
