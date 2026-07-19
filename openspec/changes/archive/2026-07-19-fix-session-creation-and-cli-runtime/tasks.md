- [x] 1. Session creation UI
  - [x] 1.1 Remove direct `cli` and `native-desktop` choices from the create-session page.
  - [x] 1.2 Fix local/remote choice contrast in selected and unselected states.
  - [x] 1.3 Generate default session name as `<current-folder-name>-<timestamp>`.
  - [x] 1.4 Normalize Windows extended-length path prefixes for display/name derivation.

- [x] 2. Opening-method management
  - [x] 2.1 Allow ordering/default switching without launching external software.
  - [x] 2.2 Persist changed order/default through the existing service boundary.

- [x] 3. Codex session runtime
  - [x] 3.1 Reproduce or trace the no-response path for Codex-created sessions.
  - [x] 3.2 Fix Codex invocation/stdin/output parsing so a sent message yields response events or a surfaced error.
  - [x] 3.3 Add regression coverage for Codex message routing.

- [x] 4. Session page CLI metadata
  - [x] 4.1 Display the selected CLI tool icon on the session page.
  - [x] 4.2 Cover icon rendering with existing mock/Web data.

- [x] 5. Validation
  - [x] 5.1 Run targeted frontend tests.
  - [x] 5.2 Run targeted Rust tests/checks where touched.
  - [x] 5.3 Run OpenSpec validation for this change.

- [x] 6. Feedback follow-up fixes
  - [x] 6.1 Strengthen local/remote selected-state contrast beyond utility class ordering.
  - [x] 6.2 Label the known project list as recently opened projects.
  - [x] 6.3 Reposition the session right-click menu near the pointer and keep it inside the viewport.
  - [x] 6.4 Auto-collapse the folder opener dropdown when pointer interaction moves elsewhere.
  - [x] 6.5 Make folder opener dropdown selection update the default opener without launching external software.

- [x] 7. Runtime and session page follow-up fixes
  - [x] 7.1 Diagnose Codex completed-with-empty-output using local runtime logs and CLI help output.
  - [x] 7.2 Add Codex final-message file fallback and broaden structured JSON text parsing.
  - [x] 7.3 Fix Codex stdin prompt argument shape for current `codex exec` and resume flows.
  - [x] 7.4 Resolve OpenCode npm Windows shim to the real executable for generation.
  - [x] 7.5 Render session CLI icons with cc-switch-style branded app icons.
  - [x] 7.6 Align the session context menu to the selected session row with viewport flipping.

- [x] 8. Shell and session page follow-up fixes
  - [x] 8.1 Strip Windows extended-length path prefixes before launching or resetting shell cwd.
  - [x] 8.2 Normalize project grouping labels that are derived from session folders.
  - [x] 8.3 Reposition the session context menu beside the selected session row with viewport clamping.

- [x] 9. OpenCode and CLI identity follow-up fixes
  - [x] 9.1 Parse current OpenCode JSON output fields including `sessionID`, `part.text`, and `step_finish`.
  - [x] 9.2 Reposition the session context menu near the right-click pointer with viewport clamping.
  - [x] 9.3 Use branded CLI icons in create session, Agent management, CLI management, and CLI parameter management surfaces.
  - [x] 9.4 Make the selected CLI in create session visibly explicit.
