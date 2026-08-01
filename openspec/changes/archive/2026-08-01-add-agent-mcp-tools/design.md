## Context

**Fixed tool catalog today.** `tool_catalog()` (`agent_runtime::application::tool_catalog.rs`) is a pure, zero-argument function returning `Vec<ToolDefinition>` where `ToolDefinition { name: &'static str, description: &'static str, input_schema: Value }` (`application/models.rs:362-366`) — both `name` and `description` are compile-time string literals today, not owned strings. It's called from three places: the two wire-format modules' own unit tests (`anthropic_provider.rs:293`, `openai_compatible_provider.rs:264`, unaffected by this change) and, critically, `execute()` in `infrastructure/api_process_adapter.rs:383`, which is the one live call site — its result is threaded into every provider request built for the rest of that generation's tool-use loop.

**Dispatch today.** `execute_tool_call(name, input, workspace_folder, cancelled, agent_id, memories)` (`api_process_adapter.rs:970-1015`) special-cases `remember` before the workspace-folder gate (it has no folder dependency), then requires a folder for everything else, then matches `SHELL_TOOL_NAME`/`FILE_TOOL_NAME` by name with a fail-closed `other => ToolExecutionOutcome { output: format!("Unknown tool \"{other}\"."), is_error: true }` fallback (line 1010-1013) — the natural interception point for a new tool source. `risk_tier_for(tool_name, input)` (`tool_catalog.rs:72-84`) already fails closed to `RequiresApproval` for any name it doesn't explicitly recognize — MCP tool names, once prefixed (Decision 3), fall through this existing catch-all with zero code changes.

**MCP subsystem today.** `McpConnectionPort` (`tooling::mcp::application::ports.rs:49-52`) is the entire trait: `async fn test(&self, server: &ServerConfiguration) -> ConnectionOutcome`. Its real implementation, `RmcpConnectionAdapter` (`infrastructure/connection_adapter.rs`), is a fully self-contained one-shot per call: build a stdio (`TokioChildProcess`) or SSE (`StreamableHttpClientTransport`) transport, `().serve(transport).await`, `client.peer().list_all_tools().await`, `client.cancel().await` — wrapped in a 15s `tokio::time::timeout` (`MCP_TEST_TIMEOUT`). `StreamableHttp` transport is a reserved, always-erroring stub. Visibility is `McpServerRepository::list_visible(&self, current_project_path: &str)` (`application/ports.rs:8-11`, `pub(crate)`) — SQL `WHERE scope = 'user' OR (scope = 'project' AND project_path = ?1)` — filtered separately in Rust to `.filter(ServerConfiguration::is_active)`; this is the exact rule `bootstrap/managed_mcp_relay.rs`'s `InvocationScopedMcpRelayAdapter::prepare_servers` already uses for CLI agents, calling `list_visible(project_path.unwrap_or_default())` directly. Each server's last-tested tool list is durable: `ServerStatus { name, connection_status, tools: Vec<ToolDescriptor>, last_connected, error, duration_ms }` (`domain/mod.rs:264-271`), readable via the already-published `McpApi::server_status(name)`.

**Cross-context boundary is enforced by a fitness test, not just convention.** `native_context_dependencies_point_inward` (`tests/architecture.rs:520`) parses every `.rs` file under a context's `domain/` or `application/` directory and flags any `use` of `crate::contexts::<other-context>::<anything except api>::*` (`imports_private_cross_context_module`, line 155-162) or of `crate::contexts::<own-context>::infrastructure::*` from that context's own `application`/`domain` code (`is_forbidden_outer_layer`, line 134-153) — i.e. **application-layer code cannot depend on infrastructure, even its own context's infrastructure, let alone another context's.** `infrastructure/` files are unscoped by this specific check, but every existing `agent_runtime::infrastructure` gateway already treats the other context's `api.rs` as the only door in practice: `RuntimeAgentSkillAdapter` wraps `tooling::skills::api::SkillApi` (`skill_gateway.rs:1-4`), `RuntimeAgentCliProfileAdapter` wraps `tooling::cli::api::CliApi`, `RuntimeAgentAvailabilityAdapter` wraps `tooling::sdk::api::SdkApi`. The one exception in the whole codebase, `bootstrap/managed_mcp_relay.rs`, reaches into `tooling::mcp::application`/`infrastructure` directly — but `bootstrap` is the composition root, outside any context, and is a one-way exception (contexts may not depend back on `bootstrap`). This change is new `agent_runtime` business logic, not composition-root wiring, so it follows the gateway convention, not the bootstrap exception.

**A live tool-invocation client already exists in the vendored dependency**, just unused: `rmcp = "2.2.0"`'s `Peer<RoleClient>::call_tool(&self, params: CallToolRequestParams) -> Result<CallToolResult, ServiceError>` (confirmed by reading `rmcp-2.2.0/src/service/client.rs:379`, the same file `list_all_tools` (line 392) already lives in). `CallToolRequestParams::new(name).with_arguments(JsonObject)` builds the request; `CallToolResult { content: Vec<ContentBlock>, structured_content: Option<Value>, is_error: Option<bool>, meta: Option<Meta> }` (`model.rs:2931-2944`) is the response — per rmcp's own doc comment on `CallToolResult::error()`, a tool that ran but failed is `Ok(CallToolResult { is_error: Some(true), .. })`, not `Err(ServiceError)`; `Err` is reserved for protocol/transport-level failure. This maps cleanly onto `ConnectionOutcome`'s own existing philosophy of representing failure as *data*, not a Rust `Result` error, at the connection-adapter layer.

**The sync/async bridge is resolved, not speculative.** The entire native-API-agent tool-execution path (`RuntimeAgentApiAdapter::monitor_generation` → `execute` → `execute_tool_call`) runs on a plain `std::thread::spawn` OS thread using `reqwest::blocking::Client`, with zero `.await` anywhere in that call graph. `tauri::async_runtime::block_on` (confirmed by reading the vendored `tauri-2.11.5/src/async_runtime.rs:272-275`) taps the same lazily-initialized global runtime that `tauri::async_runtime::spawn`/`spawn_blocking` already use elsewhere in this codebase — calling `block_on` from a thread that is not itself one of that runtime's own worker threads (exactly this case) is tokio's documented-safe pattern.

## Goals / Non-Goals

**Goals:**
- Native API agents can discover MCP-sourced tools (from visible, active MCP servers) alongside their fixed `shell`/`file`/`remember` catalog, and invoke them through a real, live MCP `tools/call`.
- Reuse every applicable existing mechanism rather than rebuilding it: `McpServerRepository::list_visible` + `.filter(is_active)` for scoping, `ServerStatus.tools` for cached catalog data, `McpConnectionPort`'s one-shot connect/call/disconnect shape, the `agent_runtime` gateway-through-`api.rs` convention, `risk_tier_for`'s existing fail-closed default, `resolve_system_prompt`'s existing best-effort-degrade philosophy for auxiliary/enrichment ports.
- Keep `tool_catalog()` a pure function; merge in MCP entries only at the one live call site (`execute()`), not by changing what "the fixed catalog" means.

**Non-Goals:**
- Any change to CLI-based agents' own MCP relay mechanism (`InvocationScopedMcpRelayAdapter`) — untouched.
- Connection pooling/caching for live tool calls — one-shot only, exactly like `test()`.
- Finishing the `StreamableHttp` transport stub.
- Any UI change to the MCP servers management page (add/edit/test/remove already fully built).
- A new agent-to-MCP-server binding table — visibility stays purely `(scope, project_path, active)`-based, identical for every agent that can see a given project, matching how the CLI relay already treats it.
- Telemetry/observability instrumentation specifically for MCP tool calls — `execute_shell`/`execute_file` don't get dedicated telemetry either; the whole generation is already wrapped by `execution_observability`, and per-tool-call MCP telemetry can be added later if it proves needed.

## Decisions

### 1. New `agent_runtime`-owned port `AgentMcpToolPort`, implemented by a new gateway adapter wrapping `McpApi` — mirrors `AgentSkillPort`/`RuntimeAgentSkillAdapter` exactly

```rust
// agent_runtime::application::ports.rs
pub(crate) trait AgentMcpToolPort: Send + Sync {
    fn catalog_entries(&self, project_path: &str) -> Result<Vec<ToolDefinition>, AgentRuntimeApplicationError>;
    fn call_tool(&self, project_path: &str, tool_name: &str, arguments: &Value) -> AgentToolCallOutcome;
}
```

`RuntimeAgentMcpToolAdapter` (new, `agent_runtime::infrastructure::mcp_tool_gateway.rs`) wraps `tooling::mcp::api::McpApi`, following `RuntimeAgentSkillAdapter`'s documented rationale verbatim: depend on another context's API through an `agent_runtime`-owned port, not that context's types directly. Wired into `RuntimeAgentApiAdapter` as `mcp: Arc<dyn AgentMcpToolPort>`, alongside the existing `skills`/`memories` fields, threaded down into `execute()`/`execute_tool_call` the same way.

**Why:** this is the only pattern used anywhere in this codebase for `agent_runtime` reaching another context's capability — not a new convention, a repetition of one already used four times (skills, CLI profiles, prompt hooks, SDK availability).

**Alternative considered:** have `execute()`/`execute_tool_call` (both `infrastructure`) import `tooling::mcp::api::McpApi` directly, skipping a new port/adapter — technically legal (the fitness test only scopes `domain`/`application`), but breaks the one exceptionless convention every existing gateway follows, and would make `RuntimeAgentApiAdapter` harder to test in isolation (today's tests already fake `AgentSkillPort`/`AgentMemoryPort` — a new untestable direct dependency would be the odd one out).

### 2. `AgentMcpToolPort`'s methods return an application-layer outcome type, not infrastructure's `ToolExecutionOutcome`

`ToolExecutionOutcome` is defined in `agent_runtime::infrastructure::tools::mod.rs` — an infrastructure-layer type. A port trait declared in `application/ports.rs` cannot mention it: `is_forbidden_outer_layer` forbids `Layer::Application` code from importing `crate::contexts::agent_runtime::infrastructure::*`, even within the same context. So `call_tool` returns a new, structurally-identical `AgentToolCallOutcome { output: String, is_error: bool }` defined in `application/models.rs`, and `execute_tool_call`'s new MCP dispatch arm does a trivial field-for-field conversion into `ToolExecutionOutcome`.

**Why:** this is a real architectural constraint (verified against the fitness test's actual logic, not assumed), not a style preference — the alternative would fail `cargo test --test architecture`.

**Alternative considered:** move `ToolExecutionOutcome` itself into `application/models.rs` so every tool-execution path shares one type. Rejected as unnecessarily invasive for this change — it would touch `execute_shell`/`execute_file`/`execute_remember` and every existing test asserting against `ToolExecutionOutcome`, for a benefit (one fewer near-duplicate struct) this phase doesn't need.

### 3. Tool-name prefix `mcp__<server-name>__<tool-name>`, parsed by splitting on the first `__` after stripping `mcp__`

`MCP_TOOL_NAME_PREFIX = "mcp__"` lives in `application/tool_catalog.rs` alongside `SHELL_TOOL_NAME`/`FILE_TOOL_NAME`/`REMEMBER_TOOL_NAME`. `RuntimeAgentMcpToolAdapter::catalog_entries` builds each `ToolDefinition.name` as `format!("{MCP_TOOL_NAME_PREFIX}{server_name}__{tool_name}")`; `execute_tool_call` detects the prefix with `.starts_with(...)` and passes the *whole* name through to `mcp.call_tool(project_path, name, input)`, which does the actual server/tool split.

**Why:** matches the naming convention already used by Claude Code and other MCP-integrated tools, so it reads familiarly rather than inventing new vocabulary. Splitting on the *first* `__` (not the last, not by a delimiter that could appear in either half) is unambiguous because `ServerName::parse` (`tooling::mcp::domain::mod.rs:24-36`) only accepts lowercase letters, digits, and hyphens — a server name can never itself contain `_`, so the first `__` after the prefix is always the true boundary, no matter how many further `__` sequences the remote tool's own name contains.

**Alternative considered:** a structured `ToolDefinition.source: ToolSource` enum instead of prefix-encoding into the name. Rejected — `ToolDefinition` is serialized straight into each provider's wire-format `tools` request (`build_request_body`), which only has a flat `name` string; encoding the source anywhere other than the name itself would need a second, parallel lookup table threaded through the same call chain for no behavioral benefit.

### 4. Call-time dispatch re-validates server visibility; it does not trust the name embedded in the tool call

`McpApi::call_tool(project_path, server_name, tool_name, arguments)` re-derives the visible+active server set the same way catalog-building did (`list_visible(project_path).filter(is_active)`) and looks up `server_name` in it, returning `McpApplicationError::ServerNotFound` (already an existing variant) if absent — *before* attempting any connection.

**Why:** the model only ever sees tool names VaneHub put in the catalog, but nothing stops a model from emitting a `mcp__<name>__<tool>` string for a server it was never shown (hallucination, or an adversarial prompt-injection scenario) — including, concretely, a *different* project's project-scoped server that happens to exist in this same user's SQLite database. Re-checking visibility at call time closes that gap for the same cost as the check already performed at catalog-build time, and matches this file's existing fail-closed treatment of unrecognized identifiers (unknown tool names, unrecognized file `operation` values).

**Alternative considered:** trust the embedded server name and connect directly (skip re-validation, since the catalog was already filtered). Rejected — cheap to avoid, and the failure mode it prevents (crossing a project boundary via a crafted tool name) is exactly the kind of thing this codebase already treats as worth a fail-closed default elsewhere.

### 5. Catalog entries are sourced from each server's cached `ServerStatus.tools`, never a fresh connection

New `McpApi::visible_tool_catalog(project_path) -> Result<Vec<McpServerToolEntry>, McpError>` (sync): `list_visible(project_path)?.filter(is_active)`, then `repository.status(name)?.tools` per server, flattened into `McpServerToolEntry { server_name: String, tool: ToolDescriptor }`. A server that was never tested, or whose last test failed, naturally contributes zero entries (`ServerStatus`'s tools list is empty in both cases) — no special-casing needed.

**Why:** stated in the proposal — building the catalog must not add synchronous multi-server network latency to every generation's start. The trade-off (catalog can list a tool that's gone stale since the last manual test) is explicitly accepted; the call-time path (Decision 4 + normal MCP connection failure handling) still fails safely if a stale entry no longer works.

**Alternative considered:** live-connect to every visible server at catalog-build time. Rejected per the proposal's explicit non-goal — this is a per-generation cost multiplied by every configured server, paid even when the model never calls a single MCP tool that turn.

### 6. `AgentMcpToolPort::catalog_entries` degrades gracefully on failure; `call_tool` never fails at the Rust type level

`catalog_entries` returns a `Result` (so a lookup failure is representable), but its *only* caller — `execute()`'s tool-catalog assembly at the `tool_catalog()` call site — treats `Err` exactly like `resolve_system_prompt` already treats `AgentSkillPort`/`AgentMemoryPort` failures (`api_process_adapter.rs:610-674`, explicitly documented there as "Neither source can fail the generation on lookup error — each logs its own warning and falls back to contributing nothing"): log an `AgentLogLevel::Warn` and continue with just the fixed 3-tool catalog. `call_tool` has no `Result` in its signature at all — every failure mode (server not found, connection failed, tool-level error reported by the remote server) resolves to `AgentToolCallOutcome { is_error: true, output: "..." }`, exactly matching `execute_shell`/`execute_file`/`execute_remember`'s existing infallible-signature convention.

**Why:** this is not a new philosophy — it's the same "best-effort enhancement" treatment this codebase already gives every optional, auxiliary system-prompt/tool-availability input, applied to one more.

**Alternative considered:** fail the whole generation if the MCP catalog can't be built. Rejected — MCP tools are additive on top of an already-usable fixed catalog; losing them for one turn because of a transient MCP-side issue shouldn't block the shell/file/remember tools that still work fine.

### 7. `risk_tier_for` needs no code change — MCP tool calls already fall into `RequiresApproval` through the existing catch-all

`risk_tier_for`'s `_ => ToolRiskTier::RequiresApproval` (`tool_catalog.rs:82`) already applies to any name that isn't literally `"file"` or `"remember"`. Because MCP catalog entries are always prefixed (Decision 3), an MCP tool name can never equal either literal, so it always falls through to the existing fail-closed default — unconditional `RequiresApproval`, with zero production code change. This still needs an explicit regression test (`risk_tier_for` called with an `mcp__...` name) so the behavior is locked in, not just an accident of the current match arms.

**Why:** confirms the proposal's stated requirement is already satisfied by this file's existing design, rather than needing a new branch that could accidentally diverge from it later.

### 8. Non-text MCP result content renders as a placeholder marker, not binary-to-text conversion

`CallToolResult.content: Vec<ContentBlock>` may contain `Text | Image | Audio | Resource | ResourceLink` blocks. `RmcpConnectionAdapter::call_tool` joins all `ContentBlock::Text` blocks' text with newlines; any non-text block becomes a short marker like `[image content omitted]` in the same joined string.

**Why:** `ToolExecutionOutcome.output`/`AgentToolCallOutcome.output` is a plain `String` today for every existing tool — there is no existing image/binary-content channel anywhere in the native tool-use loop to extend instead, and building one is out of scope for this phase.

**Alternative considered:** fail the call entirely if any non-text block is present. Rejected — a tool can legitimately return a mix of text and, say, one resource link; discarding the whole result over one block the text channel can't represent is worse than a clearly-labeled omission.

### 9. Bootstrap wiring is additive: `McpApi` flows into `AgentRuntimeDependencies` using the exact instance already built for the CLI relay and MCP settings commands

`bootstrap/runtime.rs` already constructs `mcp_api` (line 111-115) before calling `assemble_agent_runtime_api` (line 154+). Add `mcp: McpApi` to `AgentRuntimeDependencies` (`bootstrap/agent_runtime.rs:49-61`) and pass `mcp: mcp_api.clone()` at the call site — `McpApi` is already `Clone` (`#[derive(Clone)]`), and every other cross-context dependency in that struct is already threaded in exactly this way (`skills: skill_api.clone()`, etc.).

**Why:** no new construction, no reordering — `mcp_api` already exists at the right point in the bootstrap sequence for entirely unrelated reasons (MCP settings commands, CLI relay).

## Risks / Trade-offs

- **[Risk]** A fresh subprocess (stdio) or HTTP handshake (SSE) is paid on *every* tool call, not just every connection test — higher latency per MCP tool call than a pooled connection would give. **Mitigation:** identical latency profile to the already-shipped, already-accepted `test()` path; a tool call is already a multi-second, approval-gated, user-visible operation, so a sub-second connect cost is proportionally small. Revisit pooling only if this proves disruptive in practice.
- **[Risk]** Catalog entries can go stale relative to the live server (a tool renamed/removed since the last "Test Connection"). **Mitigation:** explicitly accepted per Decision 5; the call-time path still fails safely (`is_error: true`, the model sees and can react to the failure in-conversation) rather than crashing anything.
- **[Risk]** `tauri::async_runtime::block_on` is called once per MCP tool invocation from the tool-execution OS thread, potentially several times within one generation's tool-use loop. **Mitigation:** confirmed safe — not called from inside the shared runtime's own worker thread; each call is bounded by the same 15s timeout `test()` already uses, so a hung MCP server can't hang the generation indefinitely.
- **[Trade-off]** No per-agent opt-out from MCP servers — every native API agent that can see a project sees the same MCP tools every other agent (and every CLI agent) sees for that project; a user who wants to scope MCP access more tightly per-agent has no lever this phase. **Mitigation:** matches the standing non-goal (no new binding table) and this session's established "revisit only if the friction proves real" pattern; the existing per-server active/inactive toggle remains the coarse-grained lever available today.

## Migration Plan

Purely additive: one new port trait + one new gateway adapter + one new `application/models.rs` struct (`AgentToolCallOutcome`) + one new `AgentRuntimeApplicationError` variant (`Mcp(String)`, mirroring `Skill`/`Memory`) in `agent_runtime`; one extended trait method (`McpConnectionPort::call_tool`) + two new `McpApplicationService`/`McpApi` methods (`visible_tool_catalog`, `call_tool`) + one new domain struct (`ToolCallOutcome`) in `tooling::mcp`; one new `AgentRuntimeDependencies` field. No schema changes, no changes to `test()`, the MCP relay, or the MCP settings UI. `ToolDefinition.name`/`.description` widen from `&'static str` to `String` — the only source-breaking change, fully contained to `tool_catalog.rs`'s three literal-construction sites (`.to_string()` added) and any pattern match that assumed `&'static str` specifically (none found — all existing consumers already work through `&str`/JSON serialization, which is agnostic to ownership).
