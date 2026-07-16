## 1. Domain Model and Persistence

- [x] 1.1 Add TypeScript Skill domain types for scopes, metadata, mount paths, bindings, drift issues, sync reports, and migration reports.
- [x] 1.2 Add SQLite migrations for Skills, Skill Agent bindings, Agent mount paths, built-in deletion markers, and drift/sync bookkeeping.
- [x] 1.3 Add Rust domain structs matching the frontend Skill contracts with `Result<T, String>` command-safe error conversion.
- [x] 1.4 Implement fixed path resolution for global Skills under the user-home VaneHub directory and workspace Skills under `<workspace>/.vanehub/skills`.

## 2. Skill Metadata and Built-ins

- [x] 2.1 Implement `SKILL.md` frontmatter parsing and validation for `id`, `name`, `description`, `category`, `version`, and `triggers`.
- [x] 2.2 Enforce immutable Skill ids during update operations.
- [x] 2.3 Add six built-in Skill seed definitions and standard `SKILL.md` contents.
- [x] 2.4 Implement idempotent built-in initialization with deleted built-in markers.
- [x] 2.5 Implement restore flow for deleted built-in Skills.

## 3. Mounting, Drift, and Sync

- [x] 3.1 Implement registered-Agent mount path defaults and editable persisted mount path overrides.
- [x] 3.2 Implement symlink or directory-link mount creation, managed link detection, and managed link removal.
- [x] 3.3 Implement Agent mount path migration when a mount path changes, returning migrated, removed, overwritten, backed up, and failed entries.
- [x] 3.4 Implement Skill Agent binding updates that mount or unmount enabled Skills for selected Agents.
- [x] 3.5 Implement external Skill import by copying a valid external Skill directory into the selected managed source directory.
- [x] 3.6 Implement drift detection across SQLite records, source `SKILL.md`, and Agent mount paths.
- [x] 3.7 Implement backup-and-overwrite drift synchronization with structured sync reports.

## 4. Tauri Commands and Frontend Service Boundary

- [x] 4.1 Extend `AgentService` with Skill list, preview, create, update, delete, import, restore, enable, binding, mount path, drift, sync, and workspace directory picker methods.
- [x] 4.2 Implement matching methods in `tauri-agent-client.ts` using Tauri commands only inside the adapter.
- [x] 4.3 Implement matching methods in `web-agent-client.ts` with deterministic in-memory mock Skill data and simulated filesystem results.
- [x] 4.4 Add and register Rust Tauri commands for all Skill operations.
- [x] 4.5 Ensure React components do not import Tauri APIs or perform direct filesystem work.

## 5. Skills Settings UI

- [x] 5.1 Rewrite `src/settings/pages/skills-page.tsx` as a service-backed container that loads Agents, mount paths, Skills, stats, drift, and sync state.
- [x] 5.2 Add `SkillStatsCards` for all, enabled, and mounted Skill counts.
- [x] 5.3 Add `SkillAgentMountPathsPanel` with editable Agent mount paths and migration result display.
- [x] 5.4 Add `SkillScopeTabs` with global/workspace switching and workspace directory picker integration.
- [x] 5.5 Add `SkillFilterToolbar` for category filtering and keyword search.
- [x] 5.6 Add `SkillCardList` with enable toggles, Agent binding checkboxes, source badges, preview, edit, and delete actions.
- [x] 5.7 Add `SkillDialogs` for `SKILL.md` preview, create, edit, external import, and built-in restore flows.
- [x] 5.8 Add `SkillDriftBanner` with issue counts, sync action, and sync report display.
- [x] 5.9 Add bottom summary statistics for the active scope and filtered Skill list.

## 6. Tests and Verification

- [x] 6.1 Add Rust tests for metadata parsing, built-in seed idempotency, built-in restore, mount path migration, drift detection, and backup-and-overwrite sync.
- [x] 6.2 Add frontend unit tests for service adapter contracts and Web/mock Skill behavior.
- [x] 6.3 Add Playwright coverage for scope switching, workspace directory selection, filtering, Skill card controls, Agent mount path editing, drift sync, and built-in restore.
- [x] 6.4 Run `npm run test`.
- [x] 6.5 Run `npm run build`.
- [x] 6.6 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 6.7 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 6.8 Run `openspec validate --specs --strict`.
