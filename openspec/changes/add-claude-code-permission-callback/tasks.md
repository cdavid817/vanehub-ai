## 1. Fault-injection PoC gate (prerequisite — must pass before Group 2 onward)

- [ ] 1.1 Build a minimal throwaway loopback HTTP server + hook wrapper spike reproducing the real `"type": "command"` `PreToolUse` contract end-to-end for one tool (e.g. Bash)
- [ ] 1.2 Simulate a server crash mid-request; confirm the wrapper's fail-closed behavior triggers correctly
- [ ] 1.3 Simulate a hung/non-responding server; confirm the wrapper's own bounded timeout resolves to Deny rather than hanging Claude Code
- [ ] 1.4 Simulate a malformed/protocol-drifted response (garbage JSON, unexpected shape); confirm the wrapper fails closed rather than crashing
- [ ] 1.5 Attempt a `"type": "http"` hook configuration against a real Claude Code interactive session specifically for `PreToolUse`, and record whether it fires — resolves the citation gap noted in design.md's Non-Goals; if it works, note it as a candidate future optimization only, do not redesign around it now
- [ ] 1.6 Record PoC findings (pass/fail per scenario, measured hook invocation latency); if any of 1.2-1.4 fail, revisit the design before continuing to Group 2

## 2. Permissions domain: claude-code principal

- [ ] 2.1 Verify `Principal::new` / `get_or_create_principal` / `find_principal` need no changes to support `agent_id = "claude-code"` (identity is already generic over agent id — confirm with a test, don't assume)
- [ ] 2.2 Add a test asserting a `claude-code` principal defaults to the configured default template like any other new principal

## 3. Permissions infrastructure: loopback HTTP bridge

- [ ] 3.1 Choose an HTTP server dependency (evaluate footprint/existing-workspace fit) and create the new infrastructure module (e.g. `contexts/permissions/infrastructure/hook_bridge/`)
- [ ] 3.2 Implement per-launch random port + token generation; write both to a discovery file at a resolved platform-appropriate path
- [ ] 3.3 Implement the request handler: parse incoming JSON, map tool name to `Action`/`Resource` per the fixed table, reject/pass-through unmapped tools, call `evaluate()`
- [ ] 3.4 Implement the `Ask` path: call `ApprovalBroker.create_pending`, block on the same resolution mechanism `agent_runtime::infrastructure::api_process_adapter` already uses for `await_approval`, return the resolved effect once available
- [ ] 3.5 Implement bearer-token validation; reject unauthenticated requests before evaluation
- [ ] 3.6 Wire server startup/shutdown into `bootstrap/permissions.rs` alongside the existing `assemble_permissions_api`
- [ ] 3.7 Tests: mapped tool resolves via `evaluate()`; unmapped tool is rejected before evaluation; missing/wrong token is rejected; `Ask` path blocks and resolves correctly on both human decision and timeout

## 4. Hook wrapper binary

- [ ] 4.1 Add a new `[[bin]]` target to the Cargo workspace for the wrapper (minimal dependencies: stdin read, JSON parse, HTTP client, discovery-file read)
- [ ] 4.2 Implement stdin parsing of Claude Code's `PreToolUse` payload; fail closed (deny, exit non-zero) on parse failure
- [ ] 4.3 Implement discovery-file lookup and the bounded-timeout HTTP call to the loopback server (timeout value per design.md D8, tuned against Group 1's PoC measurements)
- [ ] 4.4 Implement the offline/unreachable fallback: hardcoded read-only allowlist (`Read`, `Glob`, `Grep`) fails open, everything else fails closed
- [ ] 4.5 Implement response translation to Claude Code's `permissionDecision` JSON / exit-code contract
- [ ] 4.6 Tests for the wrapper's parsing, timeout, and fallback logic in isolation (no live server required)

## 5. cli_config: independent permission-hook projection

- [ ] 5.1 Add a new operation to `CliConfigApi` (or a sibling module) for installing/removing the VaneHub-owned `hooks.PreToolUse` entry, decoupled from `apply_profile`
- [ ] 5.2 Reuse the existing atomic-write/fingerprint/rollback primitives from the live-config adapter (`infrastructure/live_config.rs`) rather than duplicating them
- [ ] 5.3 Tests: install preserves unrelated hook entries and top-level fields; remove only removes the VaneHub-owned entry; malformed existing file is rejected without modification; drift during projection aborts without overwriting the external edit

## 6. permissions <-> cli_config wiring

- [ ] 6.1 Add a new cross-context port (e.g. `ClaudeCodeHookPort`) in `permissions::application::ports`, mirroring the existing `DefaultTemplatePort` pattern
- [ ] 6.2 Implement the adapter in `permissions::infrastructure` calling into `cli_config::api`
- [ ] 6.3 Wire the first-use install/remove calls to the template-assignment command path

## 7. Frontend: Agent Policies UI

- [ ] 7.1 Extend the agents list backing `agent-policies-page.tsx` (and its backing command/DTO) to include the `claude-code` principal
- [ ] 7.2 Add the first-use installation confirmation dialog, reusing the existing `ApplicationDialog` pattern already used for `requiresConfirmationToAssign`, gated separately from the trusted/yolo confirmation
- [ ] 7.3 Update the Web/mock adapter to simulate the `claude-code` principal deterministically, with no real file writes
- [ ] 7.4 Add i18n keys for the new confirmation dialog copy across all 5 locales, matching `i18n-resource-parity.test.ts`'s requirements

## 8. Verification

- [ ] 8.1 `npm run test`
- [ ] 8.2 `npm run build`
- [ ] 8.3 `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] 8.4 `cargo check --manifest-path src-tauri/Cargo.toml` and `cargo clippy --manifest-path src-tauri/Cargo.toml`
- [ ] 8.5 `openspec validate add-claude-code-permission-callback --strict`
- [ ] 8.6 Manual end-to-end check: a real Claude Code CLI session launched by VaneHub — confirm a mapped tool call (e.g. Bash) surfaces an `ApprovalCard` and resolves correctly for both Allow and Deny
- [ ] 8.7 Manual check: with VaneHub closed, confirm a plain terminal `claude` session still works for read-only tools and is denied for shell/write tools, per D5's offline fallback
