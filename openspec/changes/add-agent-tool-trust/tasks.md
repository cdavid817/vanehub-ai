## 1. Schema and storage

- [x] 1.1 Added `apply_agent_tool_trust_schema` to `schema.rs`, registered as migration version 32 exactly as planned.
- [x] 1.2 Added `auto_approve_tools: bool` to `ApiProviderConfig`.
- [x] 1.3 Updated `provider_config`'s SQL and row-mapping closure exactly as planned.
- [x] 1.4 "Migration is idempotent" is already covered by the existing generic idempotency tests (`reopening_is_idempotent_and_preserves_existing_records`, the `migration_fixture_tests` suite) — no dedicated new test needed, since `migrate()` runs the full list including migration 32 identically to every other migration. "Registered agent reads back `auto_approve_tools == false`" is covered by the 2 existing `infrastructure::tests` cases updated in this task (`api_agent_registration_round_trips_and_reports_available`, `openai_compatible_agent_registration_persists_base_url_and_reports_available`), which now assert `auto_approve_tools: false` as part of their existing `provider_config` equality checks. "Round-trips a `true` value" is deferred to and covered by section 2 (needs the setter, which doesn't exist yet).
  **Unplanned fixes required**: adding a new migration bumped the total registered-migration count from 31 to 32, which broke 5 pre-existing hardcoded-count assertions unrelated to this feature's own logic — `platform::database::mod.rs`'s 2 `migration_count` assertions and `migration_fixture_tests.rs`'s 3 `(1..=31)` assertions. Updated all 5 to 32; this is an expected, mechanical consequence of adding any new migration, not a design issue.

  Verified: `cargo test --lib` — 939 passed, 0 failed, 3 ignored (back to the pre-phase baseline, as expected — no new tests added yet in this section). `cargo check`/`cargo clippy` — 1 expected `dead_code` warning for `trusted_anthropic_config` (a test helper added for section 3's tests, unused until then).

## 2. Application service: read and set

- [x] 2.1 Added `ApiAgentGateway::set_auto_approve_tools` exactly as planned.
- [x] 2.2 Implemented on `SqliteAgentRuntimeRepository`, exactly the planned SQL/error shape.
- [x] 2.3 Added `AgentRuntimeApplicationService::set_auto_approve_tools`. **Implementation detail beyond the original plan**: re-reading the updated `AgentView` goes through `self.ports.registry.find(agent_id)` (not `api_agents.update`'s return value, since `set_auto_approve_tools`'s gateway method returns `()` — it's a narrower, single-field mutation, unlike `update`'s multi-field `AgentDefinition`-returning shape) — this is the first of the API-agent lifecycle-style operations in this codebase to need a *separate* re-fetch after the mutation.
- [x] 2.4 Added the facade method exactly as planned.
- [x] 2.5 `FakeWorld` extended with `set_auto_approve_tools_calls: Mutex<Vec<(String, bool)>>` (records calls) and `set_auto_approve_tools_failure: AtomicBool` (toggles the `AgentNotFound` path), mirroring `updated_agents`/`delete_api_agent_failure`'s existing conventions exactly. 2 new tests: enabling returns the updated `AgentView` and records the call (seeded via a dedicated `FakeWorld::new(vec![api_agent(...)])`, since this is the first API-agent test in this file to actually need `registry.find` to resolve — `update_api_agent`/`delete_api_agent`'s own tests never touch the registry at all); a forced-failure toggle surfaces `AgentNotFound`.

  Verified: `cargo test --lib agent_runtime::application::tests` — 24 passed (up from 22), 0 failed.

## 3. Tool-use loop: honoring the trust setting

- [x] 3.1 Added `requires_approval` to `tool_catalog.rs` exactly as planned; re-exported from `application::mod.rs`.
- [x] 3.2 Replaced the round-trip loop's approval check exactly as planned. **Follow-on cleanup**: `risk_tier_for`/`ToolRiskTier` were `api_process_adapter.rs`'s only external consumer of those two names — removing that one call site left both genuinely unused there (confirmed via `cargo check`'s own unused-import warning, not assumed), so the import list was updated to drop both and add `requires_approval`; `risk_tier_for`'s own `pub(crate)` visibility and its home in `tool_catalog.rs` are untouched, since `requires_approval` and its own tests still need it there.
- [x] 3.3 5 new `requires_approval` unit tests in `tool_catalog.rs` (trusted skips shell; trusted skips file-write; untrusted still requires both; trusted still requires an MCP call; the trust flag never affects tools that were already auto-approved — `remember`, file-read). One full `execute()`-level round-trip test, `execute_skips_the_approval_prompt_for_a_trusted_agents_shell_call`, using the openai-compatible interface format specifically because its endpoint is test-controllable via `base_url` (pointed at a local `http_fixture` mock server streaming a `shell` tool-call SSE response) — Anthropic's endpoint is hardcoded and has no equivalent seam. **Scope note, deliberate**: only the trusted path is exercised at the `execute()` level; a matching untrusted-path round-trip test was not built, because `execute()` would then block inside `await_approval`'s real (timeout-less) wait for an approval decision nothing in a synchronous unit test would ever send — the untrusted path is unchanged pre-existing behavior already covered by `requires_approval`'s own unit tests and by every other passing `execute_tool_call` test in this file (no test broke when the call site was rewired, which is itself evidence the untrusted path is intact). Also removed `trusted_anthropic_config`, a test helper added in section 1 that turned out to have no viable use — Anthropic's hardcoded endpoint can't be pointed at a fixture server the way the openai-compatible path can, so it was never going to be usable for a full round-trip test; keeping unused speculative test scaffolding around wasn't warranted.

  Verified: `cargo check --lib --tests` — 0 warnings (aside from the still-expected `set_auto_approve_tools`-unused pair, not resolved until section 4). `cargo test --lib` (module-scoped runs) — `api_process_adapter` 57 passed (up from 56), `tool_catalog` 18 passed (up from 9 relevant + 3 cross-matched from `add-agent-mcp-tools`, i.e. +5 new).

## 4. Tauri command

- [x] 4.1 Added `commands/agent_runtime/set_agent_tool_trust.rs` (returns `dto::AgentRegistryEntry` via the existing `mapper::agent_to_dto`, exactly mirroring `update_api_agent`'s return shape), registered in both `mod.rs` and `registry.rs`.
- [x] 4.2 Extended `dto::ApiAgentProviderConfig`/`mapper::api_agent_provider_config_to_dto` with `auto_approve_tools` (serializes as `autoApproveTools` via the struct's existing `#[serde(rename_all = "camelCase")]`) exactly as planned. Confirmed no other literal-construction sites of this DTO existed anywhere else in the crate.
- [x] 4.3 `cargo check --lib --bins --tests` — 0 warnings, confirming the command and DTO field are genuinely reachable end to end.

  Verified: `cargo test --lib` — 947 passed (up from 941), 0 failed.

## 5. Frontend

- [x] 5.1 Added `autoApproveTools: boolean` to `ApiAgentProviderConfig`.
- [x] 5.2 Added `setAgentToolTrust` to the `AgentService` interface and `tauri-agent-client.ts`, exactly as planned.
- [x] 5.3 Extended `webApiAgentProviderConfigs`' `registerApiAgent`-populated entries with `autoApproveTools: false`; implemented the mock `setAgentToolTrust` (mirrors `updateApiAgent`'s not-found error shape); extended the simulated `shell` approval block with an `isTrusted` check read from `webApiAgentProviderConfigs.get(session.agentId)?.autoApproveTools` — trusted agents publish a single `completed` tool_use event directly, skipping the `pendingMockToolApprovals` registration and the `awaiting_approval` event entirely.
- [x] 5.4 Added a new `AgentToolTrustToggle` component (its own file, `settings/pages/agents/agent-tool-trust-toggle.tsx` — not inlined into `agents-page.tsx`, which is already 458 lines, well past this project's 300-line-per-file guidance, so growing it further wasn't warranted) rendered inside the existing API-agent card block. Reads current trust state via the same `["agents", "provider-config", agent.id]` query key `AgentEditDialog` already uses for the same underlying data (natural cache sharing, no extra plumbing). Enabling calls `window.confirm` with the warning copy; disabling calls the mutation directly.
- [x] 5.5 Added the 5 new i18n keys to both `en.json` and `zh-CN.json`; both files still parse; i18n parity/guardrail tests pass.
- [x] 5.6 4 new `web-agent-client.test.ts` tests: `setAgentToolTrust` sets and clears, round-tripping through `getApiAgentProviderConfig`; a trusted mock agent's simulated shell call completes with no `awaiting_approval` event; an untrusted one still shows it (this baseline case had no prior test coverage at all before this task — `shell`'s approval simulation was never directly tested, only `remember`'s and MCP's were — added it now since this task's own edit touches that exact code path). New `agent-tool-trust-toggle.test.tsx` (3 tests, mirroring `agent-memory-panel.test.tsx`'s established `vi.spyOn(window, "confirm")` pattern exactly): untrusted status renders and enabling requires confirmation; declining the confirmation prevents the call; already-trusted status renders and disabling skips confirmation entirely.

  Verified: `npx vitest run` (full suite) — 98 files / 336 tests passed (up from 97/330). `npm run lint` — clean. `npx tsc --noEmit` — clean (required fixing 3 pre-existing `agent-edit-dialog.test.tsx` mock fixtures and 2 `web-agent-client.test.ts` equality assertions that were missing the new required `autoApproveTools` field — mechanical fallout of widening a shared type, not a logic change). `npm run build` — clean.

## 6. Verification

- [x] 6.1 `cargo test` (full, unscoped) — **947 passed, 0 failed, 3 ignored** (up from the 939-passed baseline at the start of this change; the 3 ignored are the same pre-existing spawned-only fixtures).
- [x] 6.2 `cargo clippy --lib --bins --tests --manifest-path src-tauri/Cargo.toml` — 0 warnings, no fixes needed this time.
- [x] 6.3 `cargo fmt --check --manifest-path src-tauri/Cargo.toml` — no diff (ran `cargo fmt` once to normalize two multi-line call sites, re-ran `cargo test`/`cargo clippy` afterward to confirm no behavioral difference).
- [x] 6.4 `npm run test` — 98 files / 336 tests passed. `npm run lint` / `npx tsc --noEmit` / `npm run build` — all clean.
- [x] 6.5 `openspec validate add-agent-tool-trust --strict` — valid.
- [ ] 6.6 Manual smoke test (desktop, a real API agent with a real key): enable trust, confirm a shell command and a file write both execute without a prompt; confirm an MCP tool call from the same agent still prompts; confirm plan mode still blocks shell/file-write for the same trusted agent; disable trust and confirm approval prompts return. **Deferred to the user** (manual/credential-requiring verification, same standing arrangement as prior native-agent phases).
