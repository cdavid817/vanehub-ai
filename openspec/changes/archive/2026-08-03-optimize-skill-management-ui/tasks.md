## 1. Shared Presentation Model and Localization

- [x] 1.1 Add typed global inventory view/filter/sort helpers for All Skills, stable CLI/API Agent ids, Assigned, Available, and Unassigned results with deterministic counts and ordering.
- [x] 1.2 Add helpers and tests for active-session workspace resolution, scope-aware Skill identity, Effective/Global/Project grouping, and configured, mounted, paused, and API-prompt binding labels.
- [x] 1.3 Add synchronized zh-CN, zh-TW, en, ja, and ko strings for global Agent navigation, assignment views, information-panel Skill subviews, project management, drift, dialogs, retry/empty states, and accessible labels; pass locale parity and visible-text guardrails.

## 2. Global Skill Settings

- [x] 2.1 Refactor `SkillsPage` to load and mutate only `{ scope: "global", workspacePath: null }`, removing the scope selector, workspace path state, and workspace directory picker while keeping every component within 300 lines.
- [x] 2.2 Implement the responsive CLI Parameter Management-style layout with All Skills, dynamic branded CLI Agents, dynamic API Agents, and Unassigned navigation plus selected-view counts.
- [x] 2.3 Implement selected-Agent Assigned and Available lists with granular CLI mount or API prompt assignment actions using stable Agent ids and targeted pending/error feedback.
- [x] 2.4 Implement a compact All Skills lifecycle inventory with source/version metadata, global enablement explanation, search, category/source/status filters, deterministic sorting, clear filters, result counts, and distinct true-empty/filtered-empty states.
- [x] 2.5 Move the selected CLI's mount-path editor into a default-collapsed advanced disclosure, omit it for API Agents, and keep migration failures visible.
- [x] 2.6 Reduce healthy global drift to a compact indicator while keeping issues and synchronization reports prominent, actionable, bounded, and dismissible where specified.
- [x] 2.7 Migrate global create/import/preview/edit/restore/delete flows to shared application dialogs with Edit/Preview Markdown modes, stale-hash handling, localized confirmation, focus containment/restoration, and contextual operation errors.

## 3. Session Information Panel Skill Management

- [x] 3.1 Extract the information-panel Skill pane into focused components and add keep-alive Effective, Global, and Project subviews without causing `session-info-panel.tsx` or a child file to exceed 300 lines.
- [x] 3.2 Resolve project context from `activeSession.worktreePath ?? activeSession.projectPath`, display the normalized resolved path, and provide a no-project state without a manual path field.
- [x] 3.3 Load global and workspace Skill overviews through canonical React Query keys, derive Effective Skills for the active stable Agent id, and invalidate only the affected overview after mutations.
- [x] 3.4 Render the Global subview as read-only active-Agent context with enablement/binding status and a navigation action to global Skill Settings.
- [x] 3.5 Implement compact Project inventory actions for create, import, preview, edit, enable/disable, delete, and active-Agent bind/unbind using application-level dialogs and the resolved workspace scope.
- [x] 3.6 Use CLI mount binding for CLI sessions and API prompt binding for API sessions, preserving configured-versus-active labels and never showing mount terminology for API-only Agents.
- [x] 3.7 Present project drift health, issues, one-click synchronization, backup/overwrite results, and targeted failures inside the Project subview without obscuring session information.

## 4. Runtime Boundaries and Regression Coverage

- [x] 4.1 Verify that global Settings and project information-panel operations use only existing `AgentService` methods and preserve equivalent Tauri and Web/mock behavior without direct `invoke()` calls or new persistence semantics.
- [x] 4.2 Extend Settings component and interaction tests for global-only requests, absence of scope controls, dynamic Agent navigation, Assigned/Available/Unassigned derivation, granular assignment, global lifecycle dialogs, mount disclosure, and drift priority.
- [x] 4.3 Extend information-panel tests for worktree/project path precedence, Effective/Global/Project groups, same-id cross-scope identity, read-only global rows, complete project operations, active CLI/API binding types, cache invalidation, and no-project behavior.
- [x] 4.4 Add responsive, keyboard/focus, accessible-name, both-theme, long-label, and Tauri/Web parity coverage, including Playwright interaction checks for global Settings and project information-panel management.

## 5. Verification

- [x] 5.1 Run `npm run lint`, `npm run test`, and `npm run build`, resolving all frontend failures and warnings.
- [x] 5.2 Run `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo clippy --manifest-path src-tauri/Cargo.toml`, recording the implementation verification result.
- [x] 5.3 Run `openspec validate optimize-skill-management-ui --strict` and `openspec validate --specs --strict`, resolving all change and main-spec validation failures.

## 6. Agent Assignment and Global Enablement Separation

- [x] 6.1 Restrict the mutable global Skill enablement control to All Skills, and render selected CLI/API Agent rows with Agent-specific assignment controls plus read-only global/paused status.
- [x] 6.2 Add interaction and presentation tests proving that selected-Agent rows do not expose global enablement, one Agent assignment does not affect another Agent, and All Skills enablement preserves existing assignments.
- [x] 6.3 Run the required frontend, Rust, and OpenSpec validation commands and record the delta verification result.

Verification on 2026-08-03: `npm run lint`, `npm run test` (115 files / 432 tests), `npm run build`, targeted Playwright Skill settings (3 tests), `cargo test` (1035 passed / 3 ignored), `cargo check`, `cargo clippy`, `openspec validate optimize-skill-management-ui --strict`, and `openspec validate --specs --strict` all passed.
