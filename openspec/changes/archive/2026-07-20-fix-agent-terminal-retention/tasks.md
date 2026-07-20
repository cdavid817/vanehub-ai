## 1. Runtime Fix

- [x] 1.1 Extend Agent Terminal idle cleanup threshold to two hours.
- [x] 1.2 Serialize native Agent Terminal open-or-attach handling so same-session concurrent starts attach instead of spawning duplicate CLI processes.
- [x] 1.3 Replay retained terminal output when attaching back to a live session.
- [x] 1.4 Add frontend bounded replay cache so remounting a session terminal paints prior content immediately.
- [x] 1.5 Update Agent Terminal lifecycle specs to match the new retention behavior.

## 2. Verification

- [x] 2.1 Run OpenSpec validation for the change and main specs.
- [x] 2.2 Run focused Rust tests for Agent Terminal runtime behavior.
- [x] 2.3 Run project verification as far as the local environment allows.
