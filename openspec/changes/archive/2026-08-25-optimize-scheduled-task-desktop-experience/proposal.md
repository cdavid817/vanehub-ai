## Why

The scheduled-task dialog exposes the core persistence operations, but its dense form, weak action feedback, unlocalized recurrence details, and lack of native UI coverage make it difficult to operate confidently. The current WebdriverIO dialog suite also assumes a fixed startup activity, causing false failures when the desktop client restores or selects another workspace surface.

## What Changes

- Redesign the scheduled-task dialog into a responsive, operationally dense management surface with clearer task summaries, status treatment, recurrence guidance, validation, empty/loading states, and safe per-task action feedback.
- Localize weekday and recurrence parameter labels and expose accessible names for every schedule control.
- Preserve loaded tasks during refresh and prevent duplicate or conflicting create, enable, disable, and delete mutations.
- Add deterministic WebdriverIO coverage that launches the real Tauri desktop client, navigates explicitly to Scheduled Tasks, and verifies native create, persistence, enable/disable, delete, focus, and dismissal behavior.
- Make shared desktop dialog navigation independent of the activity selected at startup.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `scheduled-task-management`: Strengthen the management-dialog requirements for responsive presentation, localized and accessible recurrence controls, validation, mutation feedback, and native state-preserving behavior.
- `desktop-runtime-verification`: Require deterministic navigation and native WebdriverIO evidence for the scheduled-task management path.

## Impact

- Desktop and Web UI behavior changes because both runtimes render the shared React dialog; service interfaces remain unchanged.
- Tauri scheduled-task persistence and command names remain unchanged, while native WebdriverIO tests exercise their existing service boundary against isolated SQLite state.
- Affected areas include `src/main-layout/`, locale resources, scheduled-task component tests, and `tests/desktop/` helpers/specs/configuration.
- No new dependencies and no relaxation of the frontend/native adapter boundary.
