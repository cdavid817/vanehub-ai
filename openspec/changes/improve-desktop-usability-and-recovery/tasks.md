## 1. Workspace shell and session creation

- [x] 1.1 Audit the workspace shell breakpoints and make the session list, conversation surface, and auxiliary panels non-overlapping at supported widths.
- [x] 1.2 Add a branded startup loading surface and route the workspace help action to the user guide.
- [x] 1.3 Implement the responsive multi-Agent creation dialog with seat validation and focused keyboard interaction.
- [x] 1.4 Reposition session-creation feedback and verify it does not cover navigation or the composer.
- [x] 1.5 Keep the static application icon, spinner, and `Starting...` shell visible until React mounts without replacing it with feature-loading copy.

## 2. Human-centered workflow surfaces

- [x] 2.1 Remove Windows namespace prefixes from displayed task-board workspace paths while retaining actionable original paths.
- [x] 2.2 Refine task-board cards, grouping, empty states, actions, and narrow-width layout.
- [x] 2.3 Refine goal-center hierarchy, progress, actions, and responsive loading/error states.
- [x] 2.4 Correct settings selected-navigation clipping and remove the stable-build preview label from About.

## 3. Recovery, documentation, and update feedback

- [x] 3.1 Add a service-backed recoverability query and recovery action with matching Tauri and Web/mock adapter behavior.
- [x] 3.2 Add session context-menu recovery controls, accessible failure explanation, and history-preserving result feedback.
- [x] 3.3 Correct user-guide markup rendering and internal/external link routing, including missing-target feedback.
- [x] 3.4 Fix the About update-check failure path and provide an explicit retry action.

## 4. Verification

- [x] 4.1 Add focused unit/component tests for layout, dialog validation, path display, recovery, help/documentation routing, and update retry behavior.
- [x] 4.2 Run `npm run lint:ci`, `npm run test`, and `npm run build`.
- [x] 4.3 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `npm run native:panic:check`, and `cargo test --workspace` when native recovery changes.
- [x] 4.4 Run affected Playwright and desktop verification, then validate `openspec validate improve-desktop-usability-and-recovery --strict` and `openspec validate --specs --strict`.
- [x] 4.5 Add focused startup-shell regression coverage and rerun the affected frontend, Playwright, desktop, and OpenSpec verification.
