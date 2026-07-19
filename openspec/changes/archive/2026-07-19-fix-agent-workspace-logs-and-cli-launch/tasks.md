## 1. UI Labels and Info Panel

- [x] 1.1 Rename the session tab label from Agent Terminal to Workspace / 工作区 in locale files and update affected tests.
- [x] 1.2 Rename the information-panel Agent Info tab to Basic Info / 基本信息.
- [x] 1.3 Add a compact Logs tab to the information panel that shows recent `session.agent_terminal` entries via `agentService.listSessionLogs`.
- [x] 1.4 Add a bottom multiline composer to the Workspace terminal that submits text through the existing Agent terminal input service.

## 2. Native CLI Launch Fixes

- [x] 2.1 Record terminal launch failure logs for invocation build, wrapper generation, PTY open, spawn, reader, and writer failures.
- [x] 2.1a Record terminal preflight failure logs for session validation, Agent lookup, availability validation, CLI profile loading, and lifecycle update failures.
- [x] 2.2 Resolve Windows npm-style shim executables for managed interactive CLIs when a known package binary exists.
- [x] 2.3 Add native tests for shim normalization and failure-diagnostic behavior where practical.
- [x] 2.4 Ensure CLI terminal startup uses the interactive CLI profile and is not blocked by missing managed SDK dependencies.
- [x] 2.5 Publish and consume retained terminal Running state so attached terminals do not leave the UI in Starting.

## 3. Verification

- [x] 3.1 Run OpenSpec validation for the change and main specs.
- [x] 3.2 Run focused frontend and Rust tests for the changed areas.
- [x] 3.3 Run `npm run build` and Rust checks as far as the local environment allows.
