## 1. Service Contracts and Data Model

- [x] 1.1 Extend `AppSettings` and settings normalization with a boolean `launchOnStartup` defaulting to disabled.
- [x] 1.2 Add settings service methods for database directory metadata/opening and launch-on-startup state changes.
- [x] 1.3 Update Tauri and Web/mock settings adapters with matching interfaces and desktop-only unavailable behavior where appropriate.
- [x] 1.4 Add or update frontend settings-service tests for normalization, Web/mock parity, unavailable database opening, and startup setting persistence.

## 2. Desktop Runtime Integration

- [x] 2.1 Add the official Tauri autostart plugin dependency and initialize it in the Tauri builder.
- [x] 2.2 Add Rust commands for opening the SQLite database directory and synchronizing launch-on-startup registration.
- [x] 2.3 Wire new commands through Tauri command registration and capabilities/permissions required by Tauri 2.
- [x] 2.4 Persist and load `launchOnStartup` through the existing SQLite-backed settings path.
- [x] 2.5 Add Rust tests for default startup setting, save/load behavior, database directory resolution, and command-safe error conversion.

## 3. Settings Navigation and Basic Configuration UI

- [x] 3.1 Remove SDK Dependencies from the primary settings page registry while retaining SDK page/service files.
- [x] 3.2 Reorder settings navigation so Extension Capabilities appears below higher-frequency agent, skill, and IM management entries and About remains last.
- [x] 3.3 Refine settings navigation icons and icon containers with stable rounded dimensions and semantic icon choices.
- [x] 3.4 Reorganize Basic Configuration into application preferences, startup/system behavior, data management, network proxy, logs, runtime information, storage notes, and floating assistant sections.
- [x] 3.5 Add the Data Management section with read-only database location copy, open-directory action, Web/mock limitation state, and durable diagnostic reporting on failure.
- [x] 3.6 Add the launch-on-startup control with localized labels, disabled/unavailable states, save feedback, and durable diagnostic reporting on failure.
- [x] 3.7 Move the floating assistant settings section to the bottom of Basic Configuration and polish its compact layout without changing native lifecycle behavior.

## 4. Floating Assistant Settings Performance

- [x] 4.1 Ensure the floating assistant settings section loads runtime/config state once per mount and does not poll unnecessarily.
- [x] 4.2 Keep configuration-changed event handling scoped to relevant state updates.
- [x] 4.3 Verify event subscriptions are cleaned up when the settings section unmounts or remounts.

## 5. Workspace CLI Session Icons

- [x] 5.1 Create or extend a shared agent visual identity helper keyed by stable agent ids.
- [x] 5.2 Apply CLI-specific icons and visual identity to session cards for Claude Code, Codex CLI, Gemini CLI, and OpenCode.
- [x] 5.3 Apply the same derived identity to active-session or session-adjacent workspace surfaces where a current session icon is shown.
- [x] 5.4 Add fallback behavior for unknown future agent ids without changing persisted session records.
- [x] 5.5 Add focused component tests that created sessions render the selected CLI icon identity and preserve compact layout.

## 6. Localization and Visual QA

- [x] 6.1 Add synchronized zh-CN and en translation keys for data management, database directory actions, launch-on-startup, SDK navigation hiding side effects, and updated floating assistant copy.
- [x] 6.2 Update settings navigation and Basic Configuration tests for page order, hidden SDK entry, lower Extension Capabilities entry, and new sections.
- [x] 6.3 Update workspace tests for CLI-specific session icon identity and fallback rendering.
- [x] 6.4 Run visual checks or Playwright coverage for Basic Configuration and workspace session cards in representative desktop and narrow viewports.

## 7. Verification

- [x] 7.1 Run `npm run test`.
- [x] 7.2 Run `npm run build`.
- [x] 7.3 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 7.4 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 7.5 Run `openspec validate optimize-settings-ui-and-runtime-controls --strict`.
- [x] 7.6 Run `openspec validate --specs --strict`.
