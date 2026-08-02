# Verification Report: add-agent-mcp-tools

Verified on 2026-08-02 against the archived `spec-driven` artifacts and the current implementation.

`openspec status` and `openspec instructions apply` cannot resolve an already archived change, so this verification loaded `proposal.md`, `design.md`, `tasks.md`, and `specs/agent-mcp-tools/spec.md` directly from this archive and cross-checked the current main `agent-mcp-tools` specification.

## Summary

| Dimension | Status |
|---|---|
| Completeness | 38/38 tasks complete; 5/5 requirements implemented |
| Correctness | 12/12 scenarios have implementation evidence; 10/12 have direct scenario-specific regression coverage |
| Coherence | All 9 design decisions are reflected in the implementation; no critical divergence found |

Final assessment: no critical issues. Two test-coverage warnings remain; the archived implementation is coherent and its completion record is now accurate.

## Completeness

Two stale task states were corrected during this verification:

- Task 4.1 was already implemented: `ToolDefinition.name` and `description` are owned `String` fields, fixed catalog entries construct owned strings, and both provider wire-format serializers compile and pass their tests.
- Task 7.8 was replaced with an equivalent local smoke verification. Direct Windows desktop automation could not initialize because the Computer Use native pipe was unavailable, so the smoke used the real local stdio MCP fixture together with focused native Agent and Web approval tests. This limitation is recorded rather than represented as a GUI run.

## Correctness

| Requirement | Implementation evidence | Verification evidence |
|---|---|---|
| MCP-sourced tools in the native tool catalog | `contexts/tooling/mcp/application/service.rs` builds a visible, active catalog from cached status; `contexts/agent_runtime/infrastructure/mcp_tool_gateway.rs` prefixes and maps entries; `api_process_adapter.rs` merges them with the fixed catalog | `visible_tool_catalog_includes_only_visible_active_servers_cached_tools`; `resolve_tool_catalog_merges_mcp_entries_into_the_fixed_catalog`; graceful-fallback and plan-mode tests |
| Invoking an MCP-sourced tool | `connection_adapter.rs` performs bounded one-shot stdio/HTTP `tools/call`; `mcp_tool_gateway.rs` bridges the call into Agent outcomes; `api_process_adapter.rs` dispatches prefixed names | Real `mcp_stdio_server.cjs` tests for successful output and tool-level error; timeout and connection-failure tests; Agent dispatch tests |
| MCP calls require explicit approval | `tool_catalog.rs::requires_approval` never grants the trust exemption to MCP names; `api_process_adapter.rs` awaits a decision before dispatch | MCP risk-tier tests, trusted-Agent regression test, approve/deny approval-loop tests, and Web MCP approval round trip |
| Visibility is re-validated at call time | `McpApplicationService::call_tool` re-lists visible servers, filters active servers, and rejects a missing target before calling the connection port | `call_tool_rejects_a_server_outside_the_visible_active_set_without_connecting` asserts the connection recorder remains empty |
| Web runtime simulation parity | `web-agent-client.ts` emits `mcp__mock-server__search` through the same pending approval contract used by the frontend | `simulates an MCP-sourced tool call that requires approval before completing` |

## Scenario Coverage

Directly covered scenarios:

- Visible active servers contribute cached tools.
- Untested, inactive, and out-of-scope servers contribute no tools.
- MCP catalog names cannot shadow fixed names because they use the `mcp__<server>__<tool>` prefix.
- Catalog lookup failure logs a warning and falls back to the fixed catalog.
- Successful stdio tool calls return text output.
- Connection, timeout, and remote tool-level failures return error outcomes.
- MCP calls require approval even for trusted Agents.
- Call-time visibility rejection occurs before any connection attempt.
- Web/mock MCP calls use the approval event sequence and return a completed result.

Implemented but lacking a dedicated scenario-specific regression test:

- Non-text result blocks are converted to labeled placeholders by `render_content`, but no test constructs image, audio, resource, and resource-link blocks and asserts the rendered markers.
- The shared denial branch returns `Denied by user.` before `execute_tool_call`; denial and MCP dispatch are tested separately, but no single native test uses an MCP call recorder to assert zero dispatches after an explicit denial.

## Coherence

The implementation follows the archived design:

- `agent_runtime` owns `AgentMcpToolPort`; infrastructure wraps the published `McpApi` rather than importing private cross-context layers.
- Agent and MCP application layers use separate outcome models and map them at the gateway.
- Tool identity uses the documented prefix and first-`__` split.
- Catalog generation uses cached status and degrades gracefully.
- Live calls re-check scope and active state and use one-shot connections.
- MCP calls remain fail-closed for approval.
- Non-text content is represented with placeholders.
- Bootstrap injects the already assembled `McpApi` instance.
- Architecture fitness tests pass.

## Issues

### CRITICAL

None.

### WARNING

1. **Non-text result placeholders lack a focused regression test.** Add a future test around `render_content` covering image, audio, resource, resource-link, and mixed text/non-text content.
2. **Native MCP denial lacks a single end-to-end no-dispatch assertion.** Add a future `execute`-level test that resolves an MCP approval as denied and asserts the fake MCP port's call recorder remains empty.

These are test-coverage gaps, not missing implementation. Adding production tests is outside this archive-only correction and would require a new OpenSpec change under the current project governance rules.

### SUGGESTION

- Repeat the smoke through the actual desktop window when Windows Computer Use is available and a native API provider credential can be used. Record it as supplemental manual QA rather than replacing the deterministic evidence above.

## Verification Commands

Focused verification:

- `cargo test --manifest-path src-tauri/Cargo.toml contexts::tooling::mcp::` — 37 passed, 1 ignored spawned-only relay fixture.
- `cargo test --manifest-path src-tauri/Cargo.toml contexts::agent_runtime::infrastructure::api_process_adapter::tests::` — 57 passed.
- `cargo test --manifest-path src-tauri/Cargo.toml contexts::agent_runtime::application::tool_catalog::tests::` — 14 passed.
- `npx vitest run src/services/web-agent-client.test.ts` — 48 passed.

Repository gates:

- `npm run test` — 98 files, 336 tests passed. An initial parallel run hit one unrelated Session Sidebar timeout; the isolated test passed 2/2 and the subsequent serial full run passed.
- `npm run lint` — passed.
- `npm run build` — passed; the existing Vite large-chunk warning remained and the frontend chunk gate verified 4 lazy chunks.
- `cargo test --manifest-path src-tauri/Cargo.toml` — 965 passed, 3 spawned-only fixtures ignored; 9 architecture tests passed.
- `cargo check --manifest-path src-tauri/Cargo.toml` — passed.
- `cargo clippy --lib --bins --tests --manifest-path src-tauri/Cargo.toml -- -D warnings` — passed with zero warnings.
- `cargo fmt --check --manifest-path src-tauri/Cargo.toml` — passed.
- `openspec validate --specs --strict` — passed after the archive correction.
- `openspec validate --all --strict --no-interactive` — 81 items passed, including the active change and all main specifications.
