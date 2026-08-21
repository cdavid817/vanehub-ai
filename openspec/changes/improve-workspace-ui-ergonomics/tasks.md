## 1. Workspace Column Separation

- [x] 1.1 Give the session sidebar column its own trailing gutter in `src/styles.css` and move the resize affordance inside it, so the conversation column can no longer overlap sidebar content at any supported width.
- [x] 1.2 Raise the sidebar shell into its own stacking context so a menu opened inside the sidebar renders above the conversation column.
- [x] 1.3 Keep the narrow-width and compact breakpoints consistent with the new rule and verify the trailing content of a session row stays visible at minimum sidebar width.
- [x] 1.4 Add a layout test pinning the sidebar gutter, the in-column resize affordance, and the sidebar stacking context.

## 2. Create-Session Dialog Structure

- [x] 2.1 Add a shared dialog section primitive with a localized heading and purpose line, and apply it to participant, workspace, and session-name sections.
- [x] 2.2 Give each multi-Agent seat a visible position, resolved Agent identity, and resolved expert role, and surface the cross-family reviewer constraint next to the seat it constrains.
- [x] 2.3 Add synchronized zh-CN, en, and ja locale keys for the new section headings, purpose lines, and seat labels.
- [x] 2.4 Update create-session dialog and seat-assignment tests for the new structure while asserting the submitted creation input is unchanged.

## 3. Help Destination and Documentation Page

- [x] 3.1 Add a `help` settings page id, navigation entry, icon, and lazy loader placed immediately before the About entry.
- [x] 3.2 Build the documentation page: import the bundled READMEs as raw text, select by active language with an English fallback, and render through the existing `RichMarkdown` component.
- [x] 3.3 Point the workspace Help activity entry at the documentation page instead of About.
- [x] 3.4 Add synchronized locale keys for the documentation page label, crumb, search placeholder, and page header.
- [x] 3.5 Add tests for language selection, English fallback, and the Help entry destination.

## 4. Work Board Presentation and Path Display

- [x] 4.1 Apply `normalizeDisplayPath()` to the work item card's project path, its hover title, and the board's project filter options while keeping the stored path as the filter value.
- [x] 4.2 Rework the work item card into a weighted layout that subordinates priority, project, sources, and due date to the title and groups card actions separately.
- [x] 4.3 Rework the stage columns to identify their stage and count, and distinguish a filtered-empty column from a genuinely empty one.
- [x] 4.4 Group the board header's search and categorical filters and indicate when filters are narrowing the board.
- [x] 4.5 Extract board presentation into components as needed to keep every production file at or below 300 lines.
- [x] 4.6 Update work board tests for normalized path display, filter values, and the revised card and column structure.

## 5. Goal Center Presentation

- [x] 5.1 Rework the goal list rows so title, derived status, and progress are scannable, status is identifiable without color alone, and selection is distinguishable by more than a border color.
- [x] 5.2 Rework the goal detail pane to separate identity and description from linked execution targets, and group every goal action together.
- [x] 5.3 Add localized empty states for no goals and no linked targets, and keep loaded content visible while a mutation is in flight.
- [x] 5.4 Preserve list, detail, and action access at compact widths.
- [x] 5.5 Add synchronized locale keys for the new Goal Center text and update Goal Center tests.

## 6. Settings Navigation Legibility

- [x] 6.1 Floor the settings navigation grid column at `minmax(0, 1fr)` and truncate long labels so a selected entry's highlight is never clipped.
- [x] 6.2 Keep the full label available to hover and assistive technology, and keep the narrow-layout horizontal presentation reachable by scrolling.
- [x] 6.3 Add a settings sidebar test covering a label longer than the sidebar width.

## 7. Toast Placement

- [x] 7.1 Anchor the toast viewport to the top center on workspace-width viewports and to a full-width top band on narrow viewports, and update the entry and exit animation direction.
- [x] 7.2 Update notification tests for the new placement and confirm stacking, dismissal, scope filtering, and bounded lifecycle are unchanged.

## 8. Session Runtime Recovery

- [x] 8.1 Add a `recover_session` operation to the agent runtime application service that cancels the generation lease and streaming messages, sets the lifecycle to `idle`, and records the outcome through the unified logging service.
- [x] 8.2 Reject archived sessions with a diagnostic and make the operation idempotent for sessions with nothing to cancel.
- [x] 8.3 Add the Tauri command, DTO, and mapper, and register it in the command registry.
- [x] 8.4 Add `recoverSession` to the frontend Agent service interface with contract-compatible Tauri and Web/mock implementations.
- [x] 8.5 Add a runtime failure banner to the session workspace that appears when the displayed session's lifecycle is `failed`, keeps the failure reason visible, offers the recovery action, and stays distinct from the crash-recovery acknowledgement notice.
- [x] 8.6 Add a recovery entry to the session list context menu for non-archived sessions that recovers without switching the active session.
- [x] 8.7 Publish a session-scoped notification for the recovery outcome and refresh the session so its lifecycle state updates without a manual refresh.
- [x] 8.8 Add synchronized locale keys for the banner, the menu entry, and the outcome notifications.
- [x] 8.9 Add Rust unit tests for the recovery transitions and frontend tests for both entry points, the archived-session case, and the outcome notification.

## 9. Embedded Terminal Frame

- [x] 9.1 Clip the Agent CLI and Shell terminal hosts to their own rounded frame so the xterm canvas overhang cannot square off the bottom corners, and record why the defect is only visible in the desktop runtime's engine.
- [x] 9.2 Verify the corner treatment on the desktop runtime rather than only in the Playwright browser engine.

## 10. Required Verification

- [x] 10.1 Run `npm run lint:ci`, `npm run test`, and `npm run build`.
- [x] 10.2 Run `npm run test:coverage` and confirm the project coverage thresholds remain satisfied.
- [x] 10.3 Run `npm run architecture:check`.
- [x] 10.4 Run `npx playwright test` for the UI behavior changes and record the result.
- [x] 10.5 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`.
- [x] 10.6 Run `npm run native:panic:check` and `npm run contracts:check`.
- [x] 10.7 Run `openspec validate improve-workspace-ui-ergonomics --strict` and `openspec validate --specs --strict`.
