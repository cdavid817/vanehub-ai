## 1. Contracts and Domain Model

- [x] 1.1 Add strict TypeScript opener ids, availability/status, preference view/mutation, refresh, and launch result types plus contract normalization tests.
- [x] 1.2 Add the Rust desktop folder-opener catalog, preference aggregate, configured/effective default rules, File Explorer invariant, and deterministic candidate ranking with pure domain tests.
- [x] 1.3 Add dedicated desktop settings repository methods and recognized keys that load defaults and atomically persist configured default plus normalized enabled ids.
- [x] 1.4 Add command DTO mappings and coherent settings events without persisting executable paths, versions, or detection state.

## 2. Windows Discovery and Launch Infrastructure

- [x] 2.1 Define fakeable desktop ports for environment variables, PATH/App Paths/uninstall-registry reads, bounded known-location enumeration, executable metadata, cache/clock, and external launch.
- [x] 2.2 Implement bounded VS Code, File Explorer, Windows Terminal, and Git for Windows discovery with source ranking, path validation, and explicit protection against classifying WSL `bash.exe` as Git Bash.
- [x] 2.3 Implement bounded IntelliJ IDEA and WebStorm discovery for registered, known-location, and JetBrains Toolbox installations with deterministic version/edition selection.
- [x] 2.4 Implement lazy cached discovery, forced refresh, partial per-opener failures, pre-launch revalidation, and redacted unified discovery diagnostics.
- [x] 2.5 Add an explicit-argument detached spawn primitive to `platform::process` with executable/current-directory validation and special-character path tests.
- [x] 2.6 Implement fixed launch plans for all six opener ids, including a compatibility-verified Git Bash working-directory strategy, without shell construction or arbitrary user arguments.
- [x] 2.7 Add unified launch diagnostics with safe opener/session/target-kind/error context and no raw normal-record paths.

## 3. Session Authorization and Native Commands

- [x] 3.1 Add a workspaces application use case that resolves authorized local roots in worktree, folder, then project order and rejects missing, deleted, and remote targets.
- [x] 3.2 Add a workspaces infrastructure gateway to the published desktop opener API so session authorization and desktop launching remain in their owning contexts.
- [x] 3.3 Register declared Tauri commands for list/refresh opener availability, get/save preferences, and open-session-folder operations with `Result<T, String>` or mapped command errors.
- [x] 3.4 Add Rust application/infrastructure/command tests for atomic rollback, fallback restoration, target priority, remote rejection, vanished executables, detached launch plans, and command DTO round trips.

## 4. Frontend Services and Runtime Parity

- [x] 4.1 Extend `AgentService` and the runtime composition with opener discovery, refresh, preference, subscription, and session-folder launch methods.
- [x] 4.2 Implement the Tauri frontend adapter invocations and strict response normalization without adding direct Tauri imports to React components.
- [x] 4.3 Implement deterministic Web/mock availability and preference persistence plus an explicit native-launch-unavailable result.
- [x] 4.4 Add adapter parity and settings-event tests covering valid preferences, invalid ids, fallback-active views, refresh preservation, and Web launch limitations.

## 5. Settings User Interface

- [x] 5.1 Resolve and document the approved branded or neutral icon mapping, add reusable icon rendering, and ensure Web/mock never loads arbitrary local executable resources.
- [x] 5.2 Add a focused Basic Settings folder-opener section that shows status/path/version/edition, locks File Explorer enabled, supports multi-select and available-default selection, and saves one aggregate.
- [x] 5.3 Add non-blocking initial detection, manual refresh, partial failure, unavailable configured-default, optimistic save rollback, and desktop-versus-Web feedback states.
- [x] 5.4 Add synchronized Simplified Chinese and English labels, descriptions, accessibility names, statuses, fallback explanations, and errors.
- [x] 5.5 Add component tests for selection invariants, unavailable defaults, refresh behavior, save failure rollback, status rendering, and Web/mock messaging.

## 6. Session Toolbar User Interface

- [x] 6.1 Refactor the session toolbar so the existing eight-tab `tablist` and a separate fixed split opener control are siblings while preserving tab keyboard behavior.
- [x] 6.2 Implement the effective-default main action and accessible opener menu with icons, current/default indication, fallback feedback, launch loading/error states, and a settings navigation action.
- [x] 6.3 Disable or explain the opener for no-session, missing/deleted local root, remote session, and no-effective-opener states without requesting native launch.
- [x] 6.4 Add component tests for default and alternate launches, menu keyboard/focus behavior, tab-list isolation, session switching, fallback feedback, and narrow-layout classes.

## 7. End-to-End and Project Verification

- [x] 7.1 Add Playwright coverage for the Web/mock toolbar menu, settings persistence, simulated native limitation, fallback presentation, and narrow viewport behavior.
- [x] 7.2 Run `npm run lint` and resolve all reported issues.
- [x] 7.3 Run `npm run test` and `npm run build` successfully.
- [x] 7.4 Run `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo clippy --manifest-path src-tauri/Cargo.toml` successfully.
- [x] 7.5 Run `openspec validate add-workspace-folder-openers --strict` and `openspec validate --specs --strict` successfully.
