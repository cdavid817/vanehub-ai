## 1. Contracts and Dependencies

- [x] 1.1 Add typed session-workspace contracts for directory entries, file/document content, Git status/diffs, log queries/exports, Shell state/events, and bounded/partial result metadata without using `any`.
- [x] 1.2 Extend `AgentService` with grouped session workspace and Shell methods, then update compile-time contract/conformance coverage.
- [x] 1.3 Select and install React-18/Tauri-2-compatible xterm, terminal fit, Markdown, and cross-platform Rust PTY dependencies using npm/Cargo lockfiles.
- [x] 1.4 Add named first-version bounds for directory entries, document discovery depth/count, file/diff size, log page size, and message-history aggregation, with unit tests for boundary values.

## 2. Native Session Root and File Inspection

- [x] 2.1 Add a native helper that resolves the canonical project/worktree/folder root from a registered session id and returns a typed unavailable result when no root exists.
- [x] 2.2 Add relative-path validation and canonical containment checks that reject parent traversal and out-of-root symlink targets.
- [x] 2.3 Implement the immediate-child directory command with hidden-entry filtering, directories-first deterministic sorting, entry bounds, and truncation metadata.
- [x] 2.4 Implement bounded document discovery for Markdown/text files with hidden-directory, depth, count, and symlink rules.
- [x] 2.5 Implement the shared 1 MiB read-only file-content command with binary, oversized, missing, and decoding outcomes.
- [x] 2.6 Add Rust tests for valid roots, missing roots, traversal, symlink escape where supported, hidden entries, sorting, truncation, binary files, and oversized files.

## 3. Native Git Inspection

- [x] 3.1 Implement structured porcelain status parsing with separate index/worktree state and modified, added, deleted, renamed, conflicted, and untracked metadata; expose binary metadata on bounded diff results.
- [x] 3.2 Implement bounded working-tree and staged diff commands that use explicit Git arguments, fixed session-root cwd, disabled external diffs, and structured file/hunk/line output.
- [x] 3.3 Implement bounded untracked-text additions against an empty file plus metadata-only results for oversized or binary content.
- [x] 3.4 Route Git command audits and failures through unified redacted logging while returning concise service errors.
- [x] 3.5 Add Rust fixture tests for non-Git roots, staged/unstaged changes, untracked files, deletes, renames, binary files, malformed diff input, and command failures.

## 4. Native Session Logs

- [x] 4.1 Implement bounded newest-first reads of valid JSON-lines entries from the active unified log and exact structured session-id filtering.
- [x] 4.2 Implement error/warn/info/debug level filters, case-insensitive redacted text search, paging metadata, and safe malformed-line skipping.
- [x] 4.3 Implement destination selection and export of only the current filtered redacted entries, including non-error cancellation behavior and no arbitrary source path input.
- [x] 4.4 Add Rust tests proving session isolation, level/search filtering, paging, malformed-line safety, cancellation, redacted export, and absence of SQLite log persistence.

## 5. Native PTY Shell

- [x] 5.1 Add a managed Shell module with one platform-default PTY child per shell id, canonical session-root cwd, lifecycle metadata, and app-managed shared state.
- [x] 5.2 Implement shell create, input, real resize, and idempotent kill commands without accepting an arbitrary executable or cwd from the frontend.
- [x] 5.3 Emit shell-id/session-id-scoped output and lifecycle events and persist only redacted lifecycle diagnostics, never raw commands or PTY output.
- [x] 5.4 Integrate Shell cleanup with session archive/delete, selected-session reset, explicit disconnect, and application exit.
- [x] 5.5 Add Rust tests for manager ownership, create failure, input/resize routing, repeated kill, session isolation, and cleanup paths without `unwrap()`/`expect()` outside tests.
- [x] 5.6 Register the session inspection, Git, log, and Shell commands/state in the Tauri application without adding feature-local logging files.

## 6. Desktop and Web Adapters

- [x] 6.1 Implement every new `AgentService` method in `tauri-agent-client.ts`, keeping all Tauri invoke, event-listener, and native destination-selection behavior out of React components.
- [x] 6.2 Implement deterministic Web/mock Files, Documents, Changes, Logs, Terminal/Report source data, and typed unsupported local-export behavior.
- [x] 6.3 Implement a clearly labelled Web Shell simulator with deterministic prompt/echo fixtures, interface-compatible input/resize/kill behavior, and no local side effects.
- [x] 6.4 Add adapter parity tests covering matching data shapes, error/unavailable states, simulated/native capability flags, event unsubscription, and cleanup.

## 7. Session Tab Container and Chat Refactor

- [x] 7.1 Extract the current center Chat presentation from the oversized `main-layout.tsx` into focused components while preserving message queries, streaming updates, draft state, composer controls, and existing E2E selectors.
- [x] 7.2 Implement the eight-tab `SessionTabs` and icon/label/badge tab bar in the specified order with accessible tab roles, keyboard navigation, and narrow-layout horizontal scrolling.
- [x] 7.3 Implement lazy first mount, CSS keep-alive visibility, Chat-only composer display, and active-session reset to a Chat-only mounted set.
- [x] 7.4 Add shared localized loading, empty, partial, unavailable, and concise-error states for session-dependent tabs.
- [x] 7.5 Keep the right Agent Info / Files / Changes panel as a compact overview and reuse service/query results without introducing a second model.

## 8. Project and Document Tabs

- [x] 8.1 Implement Files with lazy folder expansion, retained mounted-state selection, truncation markers, and bounded text/binary/oversized preview states.
- [x] 8.2 Implement Documents with bounded discovery, read-only Markdown rendering with raw HTML disabled, and whitespace-preserving plain-text viewing.
- [x] 8.3 Implement Changes status navigation and staged/working source selection with localized non-Git, binary, oversized, rename, conflict, and untracked states.
- [x] 8.4 Implement unified and split diff renderers from one structured hunk/line model while preserving selected path and practical scroll context on view changes.
- [x] 8.5 Add focused frontend tests for file-tree state, preview outcomes, Markdown safety, Git status mapping, and unified/split diff row construction.

## 9. Terminal, Shell, Logs, and Report Tabs

- [x] 9.1 Implement Terminal execution cards from selected-session tool-use blocks, localized statuses, total badge count, empty state, and visible partial-history indication.
- [x] 9.2 Implement the xterm Shell panel with theme-variable-derived colors, connection states, event subscription cleanup, fit-on-activation, real resize requests, input forwarding, CD, Clear, and Disconnect controls.
- [x] 9.3 Implement Logs with bounded paging, level filters, submitted search, safe context display, export/cancel feedback, Web unavailability, and empty/error states.
- [x] 9.4 Implement pure session report aggregation for reported input/output tokens, separate character estimates, tool ranking, status counts, and chronological activity timeline.
- [x] 9.5 Implement Report cards/charts/timeline using Tailwind and semantic theme tokens with visible partial-history indication and locale-aware dates/numbers.
- [x] 9.6 Add focused frontend tests for tool counts/cards, report aggregation, log filter state, export outcomes, Shell lifecycle UI, terminal refitting, and subscription cleanup.

## 10. Localization, Styling, and Accessibility

- [x] 10.1 Add synchronized zh-CN and en keys for all tab labels, controls, tooltips, aria labels, states, filters, errors, statistics, dates, and export/Shell capability messages.
- [x] 10.2 Extend i18n parity and visible-text guardrails to the new session workspace files and remove hard-coded user-visible strings.
- [x] 10.3 Verify every tab in `futuristic` and `minimal` styles using existing semantic tokens, Tailwind classes, stable control dimensions, readable focus states, and no React inline styles.
- [x] 10.4 Add Playwright keyboard/focus coverage for tab navigation and accessible names plus responsive coverage for tab-bar and internal panel scrolling.

## 11. Integration and Verification

- [x] 11.1 Extend Playwright main-chat coverage to prove Chat behavior and draft state survive non-Chat tab switches but reset appropriately on session changes.
- [x] 11.2 Add Playwright Web/mock flows for all eight tabs, lazy mount/keep-alive behavior, Terminal badge, Files/Documents/Changes fixtures, Logs filters, simulated Shell, and Report content.
- [x] 11.3 Run `npm run lint`, `npm run test`, and `npm run build`, fixing all failures without suppressing TypeScript errors.
- [x] 11.4 Run `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo clippy --manifest-path src-tauri/Cargo.toml`, fixing all non-test warnings/errors.
- [x] 11.5 Run the relevant Playwright E2E suite in Web/mock mode and perform a desktop PTY/file/Git/log smoke test where the environment supports Tauri.
- [x] 11.6 Run `openspec validate "optimize-session-workspace-tabs" --strict` and `openspec validate --specs --strict`, then verify the implementation and technical documentation still distinguish first-version limits from future optimizations.

## 12. Verification Warning Remediation

- [x] 12.1 Map normalized service error codes to localized workspace messages so React never displays raw native diagnostics, and add focused mapping/UI tests.
- [x] 12.2 Log Git status-preflight failures during diff inspection and replace ambiguous Git status initials with localized, conventional status presentation plus tests.
- [x] 12.3 Align successful Shell creation state semantics, persist generic warnings for kill/wait failures without breaking idempotency, and extend lifecycle tests.
- [x] 12.4 Complete Report status counts and completion timeline behavior in aggregation, UI, i18n, and tests.
- [x] 12.5 Reconcile malformed unified-log handling documentation with the tested safe-skip implementation.
- [x] 12.6 Move each new Tauri command boundary into a dedicated `commands/session_tabs` or `commands/shell` file while retaining shared domain logic, then rerun full verification.
