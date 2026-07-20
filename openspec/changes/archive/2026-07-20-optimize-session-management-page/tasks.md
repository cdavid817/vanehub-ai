## 1. Session Management State

- [x] 1.1 Add typed sidebar state for batch mode, selected session ids, list/category presentation, and managed Agent filter.
- [x] 1.2 Derive visible session collections from normal, archived, search, Agent filter, and presentation state without mutating service results.
- [x] 1.3 Ensure batch selection is cleared or narrowed when batch mode exits or visible sessions change.

## 2. Sidebar UI

- [x] 2.1 Add compact controls for batch management, list/category presentation, and Agent filtering using existing Tailwind and lucide patterns.
- [x] 2.2 Update session rows to render checkbox selection controls in batch mode and preserve existing Agent identity, pinned, archived, lifecycle, and date metadata.
- [x] 2.3 Disable ordinary session switching, context menus, and category drag actions while batch mode is active.
- [x] 2.4 Add destructive confirmation for selected-session deletion with localized selected-count copy.

## 3. Deletion Flow

- [x] 3.1 Add a model-level multi-delete handler that calls `agentService.deleteSession` for selected session ids through the existing service boundary.
- [x] 3.2 Refresh active-visible sessions, archived sessions, active-session state, workflow state, and categories after deletion attempts.
- [x] 3.3 Show localized success or failure feedback for multi-session deletion without blocking unrelated navigation.

## 4. Localization And Visual Polish

- [x] 4.1 Add synchronized zh-CN and en translations for batch mode, selected counts, presentation modes, Agent filters, confirmations, and empty states.
- [x] 4.2 Check the optimized sidebar layout for both `futuristic` and `minimal` styles using existing semantic tokens and stable control dimensions.

## 5. Tests And Validation

- [x] 5.1 Add focused tests for Agent filtering, list/category presentation, batch selection, select-visible, exit behavior, and confirmed deletion.
- [x] 5.2 Run `openspec validate optimize-session-management-page --strict`.
- [x] 5.3 Run `npm run test`.
- [x] 5.4 Run `npm run build`.
- [x] 5.5 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
