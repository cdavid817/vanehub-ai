## 1. Provider request-building: `GenerationOptions`

- [x] 1.1 Added `GenerationOptions<'a> { thinking: bool, reasoning_depth: Option<&'a str> }` plus `GenerationOptions::disabled()` to `api_process_adapter.rs`, right above the `WireFormat` struct it's coupled to.
- [x] 1.2 Widened `WireFormat.build_request_body`'s type exactly as planned.
- [x] 1.3 `anthropic_provider::build_request_body`: adds `"thinking": {"type": "adaptive"}` when `options.thinking`.
- [x] 1.4 `openai_compatible_provider::build_request_body`: adds `"reasoning_effort": "<depth>"` when `options.reasoning_depth` is `Some`, via a small `reasoning_effort(depth) -> &str` helper that folds `"max"` to `"high"`.
- [x] 1.5 Updated all call sites: `execute()`'s main-turn call (built from `request.configuration.thinking`/`.reasoning_depth`), `maybe_compact`'s summarization call (`GenerationOptions::disabled()`), the one test-only `build_request_body` call in `api_process_adapter.rs`'s own test module (`GenerationOptions::disabled()`), and every `build_request_body` call in both provider modules' unit tests (11 sites total across the two files, via a shared `no_options()` test helper plus explicit `GenerationOptions{..}` literals where a test needed non-default values).
- [x] 1.6 6 new tests: `anthropic_provider` — `request_body_enables_adaptive_thinking_when_requested`, `request_body_omits_thinking_when_not_requested`, `request_body_ignores_reasoning_depth`. `openai_compatible_provider` — `request_body_passes_reasoning_depth_through_as_reasoning_effort` (covers low/medium/high in one test), `request_body_folds_max_reasoning_depth_down_to_high`, `request_body_ignores_thinking`.

  Verified: `cargo check --lib --tests` (0 warnings — confirmed `GenerationOptions` reaches both provider modules via `super::api_process_adapter::GenerationOptions` despite `api_process_adapter` being a private `mod` declaration, since sibling-module access within the same parent only needs the *item* to be `pub(crate)`, not the declaring module to be `pub`); `cargo test --lib anthropic_provider` — 19 passed; `cargo test --lib openai_compatible_provider` — 19 passed; `cargo test --lib api_process_adapter` — 48 passed (unchanged from before this task, confirming the `GenerationOptions::disabled()` call sites didn't alter any existing behavior).

## 2. Plan-mode tool catalog

- [x] 2.1 Factored `remember_tool_definition()` out of `tool_catalog()` exactly as planned — confirmed `catalog_declares_exactly_shell_file_and_remember_tools` still passes unmodified.
- [x] 2.2 Added `plan_mode_tool_catalog()`: `file` narrowed to `enum: ["read"]` (description text also updated to say plan mode is active), plus `remember_tool_definition()`. No `shell` entry. Re-exported from `application/mod.rs`.
- [x] 2.3 Added `plan_mode_catalog_offers_only_read_only_file_and_remember`, asserting exactly 2 entries and the narrowed schema.

  Verified: `cargo test --lib tool_catalog` — 12 passed (9 pre-existing + this task's 1 new one + 2 from `add-agent-mcp-tools` matched by the same substring filter), 0 failed. `cargo check --lib --tests` shows an expected `unused import`/`dead_code` warning pair for `plan_mode_tool_catalog` — resolves once section 4 wires it into `resolve_tool_catalog`.

## 3. Plan-mode execution enforcement

- [x] 3.1 Added `plan_mode: bool` to `execute_tool_call`, plus a shared `plan_mode_denial(what) -> ToolExecutionOutcome` helper for the consistent rejection message. MCP-prefixed names are gated right where the existing MCP special-case already lives (before the folder gate, for the same folder-independence reason); shell is gated inside its own match arm rather than before the folder gate, since shell already requires a folder regardless.
- [x] 3.2 Added the `operation != "read"` gate inside the `FILE_TOOL_NAME` arm, exactly as planned.
- [x] 3.3 5 new tests: `execute_tool_call_rejects_shell_in_plan_mode`, `execute_tool_call_rejects_mcp_calls_in_plan_mode_without_reaching_the_port` (asserts a `FakeMcp`'s call recorder stays empty), `execute_tool_call_still_allows_file_read_in_plan_mode`, `execute_tool_call_rejects_file_write_in_plan_mode` (also asserts the file was never created on disk), `resolve_tool_catalog_returns_the_plan_mode_catalog_without_querying_mcp`. **Extended `FakeMcp`** with a `catalog_lookups: Mutex<u32>` counter (it previously only tracked `call_tool` invocations, not `catalog_entries` lookups — needed to actually prove plan mode skips the MCP catalog lookup, not just that it never calls a tool). All pre-existing non-plan-mode tests continue to pass with an explicit `false` argument added at each call site.

  Verified: `cargo test --lib api_process_adapter` — 53 passed, 0 failed (up from 48).

## 4. Wiring into `execute()`

- [x] 4.1–4.3 Implemented together with section 3 (the same edit to `execute()` threads `plan_mode` into both `resolve_tool_catalog` and `execute_tool_call`, and builds `generation_options` for the `build_request_body` call). **Refactored beyond the original plan**: rather than inlining the `GenerationOptions` struct literal and the `permission_mode == "plan"` comparison directly in `execute()`, factored both into standalone functions — `generation_options_from_configuration(&AgentChatConfiguration) -> GenerationOptions` and `is_plan_mode(&AgentChatConfiguration) -> bool` — specifically so task 4.5's test could exercise the actual derivation logic in isolation (see 4.5's note).
- [x] 4.4 Covered by section 3.3's `resolve_tool_catalog_returns_the_plan_mode_catalog_without_querying_mcp`; the non-plan-mode call sites already had explicit `false` arguments added and continue to pass unmodified.
- [x] 4.5 Implemented as 3 tests against the two new pure functions instead of a network-adjacent `execute()`-level test: `generation_options_from_configuration_reads_thinking_and_reasoning_depth`, `generation_options_from_configuration_defaults_to_disabled`, `is_plan_mode_matches_only_the_literal_plan_value`. **Deviation from the original task wording, deliberate**: the task's own phrasing already anticipated needing "a seam that inspects the built body before it would be sent" — pulling the derivation logic out into its own functions *is* that seam, and is more direct than trying to intercept a value inside `execute()`'s own body. `build_request_body`'s actual per-provider handling of `thinking`/`reasoning_depth` is already fully covered by section 1's tests; what remained untested before this task was only the `request.configuration` → `GenerationOptions`/`bool` derivation step, which these 3 tests now cover directly.

  Verified: `cargo test --lib api_process_adapter` — 56 passed, 0 failed (up from 53). `cargo check --lib --tests` — 0 warnings (confirms `plan_mode_tool_catalog`'s earlier "never used" warning from section 2 is now resolved).

## 5. Verification

- [x] 5.1 `cargo test` (full, unscoped) — **939 passed, 0 failed, 3 ignored** (up from the 924-passed baseline; +15 new tests across sections 1-4; the 3 ignored are the same pre-existing spawned-only fixtures).
- [x] 5.2 `cargo clippy --lib --bins --tests --manifest-path src-tauri/Cargo.toml` — 0 warnings. **One fix needed**: the widened `build_request_body` function-pointer type tripped clippy's `type_complexity` lint at 5 parameters; factored it into a `type BuildRequestBody = fn(...) -> Value;` alias, which clippy's own suggestion recommended.
- [x] 5.3 `cargo fmt --check --manifest-path src-tauri/Cargo.toml` — no diff (one line-length wrap needed in a test, applied via `cargo fmt`; re-ran `cargo test`/`cargo clippy` afterward to confirm no behavioral difference).
- [x] 5.4 `npm run test` — 97 files / 330 tests passed (unchanged from before this change, as expected — no frontend files were touched). `npm run lint`, `npx tsc --noEmit`, `npm run build` — all clean.
- [x] 5.5 `openspec validate add-agent-chat-configuration --strict` — valid.
- [ ] 5.6 Manual smoke test (desktop, a real Anthropic and a real OpenAI-compatible API agent configured): confirm toggling "Extended Thinking" for the Anthropic agent produces visible thinking content; confirm selecting a reasoning depth for the OpenAI-compatible agent changes provider-visible behavior (e.g. response latency/quality on a reasoning-heavy prompt, or provider-side logs if available); confirm plan mode prevents a native API agent from running a shell command or writing a file, while still allowing it to read files and save memories. **Deferred to the user** (manual/credential-requiring verification, same standing arrangement as prior native-agent phases).
