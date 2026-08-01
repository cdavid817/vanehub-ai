## 1. Loop definition/start eligibility (`validate_agent`)

- [x] 1.1 Add `api_agents: Arc<dyn ApiAgentGateway>` to `LoopApplicationPorts` (`src-tauri/src/contexts/agent_runtime/application/loop_service.rs`).
- [x] 1.2 Rewrite `validate_agent`: if the resolved agent supports `InteractionMode::Cli`, keep the existing `ensure_selectable(InteractionMode::Cli)` path unchanged. Otherwise call `ensure_selectable(InteractionMode::Api)`, then look up `self.ports.api_agents.provider_config(agent_id)?`, and reject with a clear validation error (via the file's existing `loop_validation(...)` helper) unless `auto_approve_tools` is `true`. Treat a missing provider-config row the same as untrusted (fail closed).
- [x] 1.3 In `src-tauri/src/bootstrap/agent_runtime.rs`, change the `LoopApplicationPorts { registry: repository, ... }` construction (~line 217) from a move to `repository.clone()`, and add `api_agents: repository.clone()`.
- [x] 1.4 In `loop_service_tests.rs`, add an `ApiAgentGateway` implementation to `FakeWorld` (`provider_config` looks up trust by id; `register`/`update`/`delete`/`set_auto_approve_tools` are `unreachable!()`, matching this file's existing convention for unused port methods), and add `api_agents: self.clone()` to `FakeWorld::service()`. Added `api_agent(id)` fixture helper alongside the existing `agent()` CLI-agent helper, plus `FakeWorld::new_with_trust(...)` (delegated to by the existing `FakeWorld::new`, so all 3 pre-existing call sites are untouched) to inject which agent ids are trusted.
- [x] 1.5 New tests: `create_definition_accepts_a_trusted_api_agent_as_worker_or_verifier`, `create_definition_rejects_an_untrusted_api_agent_as_worker_or_verifier`, `manual_start_accepts_a_trusted_api_agent_as_worker_or_verifier`, `manual_start_rejects_an_untrusted_api_agent_as_worker_or_verifier`. CLI agent validation is unchanged (all 3 pre-existing tests untouched); the "agent supporting neither mode" / missing-agent case remains covered by the pre-existing `manual_start_validates_before_creating_run_or_operation`.

Verify: `cargo test --lib loop_service`.

## 2. Role-session interaction-mode plumbing

- [x] 2.1 Add `interaction_mode: InteractionMode` to `LoopRoleSessionRequest` (`src-tauri/src/contexts/agent_runtime/application/loop_models.rs:225-233`).
- [x] 2.2 Add `registry: Arc<dyn AgentRegistryRepository>` to `LoopWorkerApplicationPorts` (`loop_worker.rs`) and to `LoopVerifierApplicationPorts` (`loop_verifier.rs`).
- [x] 2.3 In `LoopWorkerApplicationService::launch_iteration` (`loop_worker.rs`), resolve `request.definition_snapshot.worker_agent_id`'s `InteractionMode` via the new registry port (prefer `Cli` if supported, else `Api`, matching `validate_agent`'s rule) before constructing `LoopRoleSessionRequest`; set the new field.
- [x] 2.4 Same change in `LoopVerifierApplicationService::start` (`loop_verifier.rs`) for `request.definition_snapshot.verifier_agent_id`.
- [x] 2.5 In `sessions_gateway.rs`'s `create_worker_session`/`create_verifier_session`, replace the hardcoded `InteractionMode::Cli.as_str().to_string()` with `request.interaction_mode.as_str().to_string()`.
- [x] 2.6 In `bootstrap/agent_runtime.rs`, add `registry: repository.clone()` to the `LoopWorkerApplicationPorts { ... }` construction (~line 236) and `registry: repository` (last usage in the function, so a plain move) to the `LoopVerifierApplicationPorts { ... }` construction (~line 253).
- [x] 2.7 `WorkerWorld` (`loop_worker_tests.rs`) and `VerifierWorld` (`loop_verifier_tests.rs`) each gained an `agent_mode: Mutex<Option<InteractionMode>>` field (`None` defaults to `Cli`, matching every pre-existing test unchanged) and an `AgentRegistryRepository` impl that builds a fixture `AgentDefinition` for the configured mode. `OrchestratorWorld` (`loop_orchestrator_tests.rs`, which doesn't test mode resolution itself) got a simpler fixed-`Cli` `AgentRegistryRepository` impl, just enough to satisfy the new port dependency.
- [x] 2.8 `create_worker_session`/`create_verifier_session` in both fake worlds now assert the received `interaction_mode` matches the configured mode (so every pre-existing test implicitly re-asserts `Cli` unchanged). New tests: `worker_iteration_for_an_api_agent_resolves_api_interaction_mode`, `start_for_an_api_agent_resolves_api_interaction_mode` — both set `agent_mode = Some(Api)` and confirm it flows through.
- [x] 2.9 `sessions_gateway.rs` has no dedicated unit tests of its own (a thin pass-through adapter with no existing `mod tests`) — nothing to update there; its correctness for this change is covered by the `loop_worker`/`loop_verifier` tests confirming the right `InteractionMode` is placed on the request before it reaches this adapter, plus the full-suite run in section 5.

Verify: `cargo test --lib loop_worker loop_verifier sessions_gateway`.

## 3. Loop role generation mode fix

- [x] 3.1 In `AgentRuntimeApplicationService::start_loop_role_generation` (`service.rs:93-122`), replace the hardcoded `interaction_mode: InteractionMode::Cli` with a lookup via `self.ports.registry.find(&session.agent_id)` (same pattern already used by `set_auto_approve_tools` in this file), preferring `Cli` if supported, else `Api`.
- [x] 3.2 New test `loop_role_generation_for_an_api_agent_session_resolves_api_interaction_mode` (`tests.rs`): registers an API agent, adds a session owned by it with `loop_ownership` set, calls `start_worker_generation` (the actual `LoopWorkerGenerationPort` entry point, not `send_message` directly, since the pre-existing loop-completion test bypasses `start_loop_role_generation` entirely), asserts the captured `generation_requests` entry's `configuration.interaction_mode == InteractionMode::Api`. The existing CLI-agent case remains covered by `loop_role_generation_delivers_one_terminal_completion_and_cancellation_wins_races`, unchanged.

Verify: `cargo test --lib agent_runtime::application::tests`.

## 4. Frontend check (no code change expected)

- [x] 4.1 Confirmed by reading `loop-definition-dialog.tsx`'s `submit()`: its `catch` block does `setError(submitError instanceof Error ? submitError.message : String(submitError))` — fully generic, no special-casing by error type. The new `validate_agent` rejection message flows through unchanged, same as every other save failure. No frontend code change needed.
- [x] 4.2 `npm run test` / `npm run lint` / `npx tsc --noEmit` — no regressions (no frontend files changed in this phase).

## 5. Full verification

- [x] 5.1 `cargo test` (full workspace, not scoped) — **965 passed, 0 failed, 3 ignored** (up from the 958-passed baseline at the start of this change; +7 new tests: 4 in `loop_service_tests`, 2 in `loop_worker_tests`/`loop_verifier_tests`, 1 in `application::tests`). Architecture fitness tests (`tests/architecture.rs`) — 9/9 passed, including `native_context_dependencies_point_inward`.
- [x] 5.2 `cargo clippy --lib --bins --tests --manifest-path src-tauri/Cargo.toml` — 0 warnings.
- [x] 5.3 `cargo fmt --check --manifest-path src-tauri/Cargo.toml` — clean (ran `cargo fmt` once to normalize 5 files' multi-line expressions after writing the new test fixtures; re-ran `cargo test`/`cargo clippy` afterward to confirm no behavioral difference).
- [x] 5.4 `npm run test` — 98 files / 336 tests passed (unchanged, no frontend files touched). `npm run lint` / `npx tsc --noEmit` / `npm run build` — all clean. `cargo check --manifest-path src-tauri/Cargo.toml` — clean.
- [x] 5.5 `openspec validate add-loop-api-agent-support --strict` — valid.
- [ ] 5.6 Manual smoke test (deferred to the user, same standing arrangement as prior native-agent phases): create a Loop definition with a trusted API agent as worker and/or verifier, start a run, confirm iterations actually execute tool calls without stalling on approval; confirm an untrusted API agent is rejected at definition-save time with a clear message; confirm a pre-existing CLI-agent Loop definition still runs unchanged.
