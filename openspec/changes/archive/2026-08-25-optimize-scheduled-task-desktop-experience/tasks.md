## 1. Baseline and UX Structure

- [x] 1.1 Run the existing native WebdriverIO dialog layer, retain failure evidence, and identify the startup-activity navigation assumption.
- [x] 1.2 Extract scheduled-task recurrence/form and list presentation into focused components while keeping service orchestration in the dialog.
- [x] 1.3 Implement responsive compact layout, scroll containment, status summaries, loading/empty/error presentation, and task content previews with semantic styles.

## 2. Interaction Quality

- [x] 2.1 Add localized weekday, interval, schedule parameter, status, and accessibility strings to every supported locale.
- [x] 2.2 Add recurrence validation and accessible field feedback that prevents invalid service submissions.
- [x] 2.3 Add refresh, create, enable/disable, and delete pending states that preserve loaded tasks and prevent conflicting mutations.
- [x] 2.4 Add or update component and recurrence tests for validation, localization, retained data, and mutation behavior.

## 3. Native Desktop Automation

- [x] 3.1 Make shared desktop helpers navigate explicitly to the required activity before using content-specific controls.
- [x] 3.2 Add an isolated scheduled-task WebdriverIO config, orchestrator layer, npm entry point, and deterministic CLI fixture environment.
- [x] 3.3 Verify dialog focus and Escape behavior plus native create, stable Agent id persistence, disable/enable, and delete lifecycle through the rendered UI.
- [x] 3.4 Run the scheduled-task and existing dialog WebdriverIO layers and inspect screenshots and unified native logs for UI or runtime failures.

## 4. Verification

- [x] 4.1 Run frontend coverage/policy, architecture, contract, build, and Playwright checks required for the UI and desktop-boundary changes.
- [x] 4.2 Run all repository Rust formatting, check, clippy, panic, and workspace test commands.
- [x] 4.3 Run the full composed desktop verification command and record the current Linux result; report other platforms as `NOT RUN`.
- [x] 4.4 Run strict validation for this change and all main OpenSpec specifications.
