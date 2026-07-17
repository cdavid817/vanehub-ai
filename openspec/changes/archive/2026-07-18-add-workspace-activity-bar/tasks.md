## 1. Activity Bar Foundation

- [x] 1.1 Add synchronized zh-CN and en translation keys for Session, Scheduled Tasks coming-soon feedback, Settings, Help, and activity-bar accessibility labels/tooltips.
- [x] 1.2 Create the icon-only workspace activity-bar presentation component with fixed-size top and bottom groups, semantic-token styling, keyboard focus states, accessible names, tooltips, and Session expanded-state semantics.
- [x] 1.3 Connect the activity-bar Settings action to the existing workspace settings callback and connect Scheduled Tasks to a localized non-blocking frontend notification without adding a route or service call.

## 2. Workspace Layout and Sidebar Behavior

- [x] 2.1 Remove Settings and Help controls and the Settings callback prop from `SessionSidebar` while preserving all existing session list, creation, grouping, archive, and context-action behavior.
- [x] 2.2 Add default-expanded session-sidebar state to `MainLayout`, render the activity bar beside the workspace grid, and toggle the sidebar from the Session activity entry independently of the information-panel state.
- [x] 2.3 Keep the session sidebar mounted inside an identified overflow-clipped wrapper and make the collapsed subtree unavailable to pointer, keyboard, and assistive-technology interaction without resetting its local view or folder state.
- [x] 2.4 Update shared workspace styles for the fixed activity-bar width, all four session/information-panel column combinations, aligned workspace height, and 200ms sidebar grid transition.
- [x] 2.5 Preserve the existing 900px information-panel behavior and bounded 640px single-column session-sidebar behavior while keeping the activity bar visible and operable.

## 3. Automated and Visual Verification

- [x] 3.1 Add component tests for activity-entry ordering, icon-only accessible labels/tooltips, top/bottom grouping, Session toggle semantics, Settings callback, Help presence, and Scheduled Tasks coming-soon feedback.
- [x] 3.2 Add workspace regression tests that collapse and expand the session sidebar, verify released grid space and independent information-panel state, and confirm sidebar activity/group/folder state survives the cycle.
- [x] 3.3 Extend Playwright coverage for Settings navigation, the non-navigating Scheduled Tasks placeholder, keyboard reachability, and responsive activity/sidebar behavior at desktop, 900px, and 640px widths.
- [x] 3.4 Visually inspect the workspace in both `futuristic` and `minimal` styles at representative widths and confirm stable icon sizing, focus/active states, aligned panel edges, readable contrast, and absence of clipping or overlap.

## 4. Project Validation

- [x] 4.1 Run `npm run lint`, `npm run test`, and `npm run build` and resolve all frontend, localization, component, and E2E-static failures introduced by the change.
- [x] 4.2 Run `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo clippy --manifest-path src-tauri/Cargo.toml` to confirm the unchanged native boundary remains healthy.
- [x] 4.3 Run `openspec validate "add-workspace-activity-bar" --strict` and `openspec validate --specs --strict`, then reconcile any planning or repository-spec validation errors before implementation handoff.
