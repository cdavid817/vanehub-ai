## 1. Verify Antigravity CLI facts against a real install

- [x] 1.1 Install `agy` on the development host and record the resolved binary path — **`%LOCALAPPDATA%\agy\bin\agy.exe`, confirming the `candidates.rs` probe path**; version installed: 1.1.11
- [x] 1.2 Determine the version-probe command and whether `agy update` exists — **`agy --version` prints a bare `1.1.11`** (no prefix to strip), and `update` exists as a subcommand
- [ ] 1.3 Capture a real `--output-format stream-json` run and record the exact `init` / `step_update` field names — **partially done**: the `result` envelope is captured verbatim from a real run (see 1.4); `init` and `step_update` payloads still need an authenticated run
- [x] 1.4 Capture the exit code and stderr of an unauthenticated non-interactive run — **exit code 1**; stderr prints the OAuth URL then `Error: authentication timed out.`; **stdout still emits a well-formed terminal event**: `{"event":"result","result":{"conversation_id":"","status":"ERROR","response":"","error":"authentication failed or timed out","duration_seconds":0,"num_turns":0,"usage":{...}}}`. The envelope is `{"event":"<kind>","<kind>":{...}}`, not a flat `{"type":...}`
- [x] 1.5 Confirm whether a catalog-legal permissive launch control exists — **yes: `--mode (accept-edits, plan)`, absent from the published docs**. `design.md` D7 and the `cli-agent-permission-launch-flags` delta are revised accordingly, and `--mode` is now a catalog parameter
- [x] 1.6 Confirm the settings document path — **`.gemini/antigravity-cli/settings.json` is hardcoded in `agy.exe`**, so `primary_path` is correct; the file is created lazily and does not exist on a fresh install. All six modelled keys (`toolPermission`, `enableTerminalSandbox`, `verbosity`, `colorScheme`, `altScreenMode`, `editorMode`) and all four `toolPermission` values appear in the binary
- [ ] 1.7 Sign in interactively and capture real model slugs from `agy models` for the `--model` catalog's known values (currently `default` only)

## 2. Widen the managed CLI tool definition

- [x] 2.1 Change `ToolDefinition.package_name` to `Option<&'static str>` and update all four existing catalog entries' construction sites
- [x] 2.2 Add a `powershell_install_url` field to `ToolDefinition` and default it to `None` for the four existing entries
- [x] 2.3 Update `install_command_for` in `tooling/cli/infrastructure/support.rs` to omit the npm fallback when no package name exists
- [x] 2.4 Update `derive_lifecycle_eligibility` so a definition with no package name and no installer for the current platform yields `Manual`, and one with a platform-appropriate installer yields `Wget`
- [x] 2.5 Add a Windows PowerShell branch to `package_adapter.rs`'s script-install path alongside the existing `bash -lc` branch
- [x] 2.6 Update `catalog_has_stable_order_ids_and_verified_sources` and `lifecycle_eligibility_follows_install_state_and_active_source` to cover the widened shape, confirming the four existing CLIs are undisturbed

## 3. Register the built-in agent

- [x] 3.1 Read the live `%APPDATA%\ai.vanehub.app\vanehub.sqlite` `schema_migrations` table and pick an unclaimed migration version number — **version 49 is already claimed by `workspace-code-index-foundation` from an unmerged worktree branch; the next migration on any branch must start at 50**
- [x] 3.2 Add the `antigravity-cli` entry to the built-in seed table in `agent_runtime/infrastructure/schema.rs`
- [x] 3.3 ~~Add an idempotent insert-or-ignore migration~~ — **not needed**: `seed_registry` runs on every bootstrap and already uses `INSERT OR IGNORE`, so the catalog entry back-fills existing databases on its own (see design D8). Covered by `back_fills_antigravity_into_a_database_seeded_before_it_existed`
- [x] 3.4 Add the `agy` catalog entry to `CLI_TOOL_DEFINITIONS` with the shell and PowerShell installer URLs and no package name
- [x] 3.5 Add `agy` discovery paths to `tooling/cli/infrastructure/candidates.rs` for the install directories confirmed in task 1.1
- [x] 3.6 Verify the Windows discovery path against the real installer output — `%LOCALAPPDATA%\agy\bin\agy.exe` confirmed on disk and pinned by `windows_candidates_cover_the_executable_named_install_directory`
- [x] 3.7 Confirm Antigravity CLI's project-level skill directory — **`.agents/skills`**, read out of the installed binary's literal template `{workspace}/.agents/skills/{skill_name}/SKILL.md` (undocumented on the web); built-in skills live at `~/.gemini/antigravity-cli/builtin/skills/<name>/SKILL.md`, the same `SKILL.md` convention the other CLIs use

## 4. Managed chat invocation and output parsing

- [x] 4.1 Add the `antigravity-cli` arm to `providers/invocation.rs` building `agy [mapped parameters] --conversation <id> -p <prompt> --output-format stream-json` with argument prompt delivery, plus the interactive arm (no id can be pre-assigned — `agy` has no documented flag to name a new conversation, so the CLI mints it and `init` reports it)
- [x] 4.2 Add the contract case to `providers/fixtures/invocations.json` pinning argument order, and the `parameter-mappings.json` case (reasoning depth clamps to `high` because `--effort` accepts only low/medium/high; permission mode projects onto `--mode`)
- [x] 4.3 Add NDJSON parsing to `providers/output.rs` keyed on the **verified** `{"event":"<kind>","<kind>":{...}}` envelope, ignoring unknown event kinds and unparseable lines
- [x] 4.4 Map `result.status` to lifecycle outcomes — **spec corrected**: the parser vocabulary has no cancelled variant (cancellation is driven by an explicit stop at the chat layer), so `CANCELED`/`INTERRUPTED` surface as non-retryable failures carrying the CLI's own wording rather than a stopped state the runtime cannot express
- [x] 4.5 Emit `init.conversation_id` as the provider runtime session id, which the existing resume path then feeds back through `--conversation`
- [x] 4.6 Map the unauthenticated signature captured in task 1.4 — its real `result` line is committed as a fixture and asserted verbatim
- [x] 4.7 Add parser unit tests: the real capture, thinking-token folding, `init` extraction, tolerance for `step_update`/unknown/non-JSON lines, and a loud failure on a non-terminal terminal status
- [ ] 4.8 Parse `step_update` into incremental output — **deliberately deferred**: its payload has never been observed, and inventing field names would produce a parser that silently drops real output. A turn still delivers the whole reply through `result.response`, so this is a streaming-fidelity gap, not a functional one

## 5. CLI configuration profiles

- [x] 5.1 Extend `cli_config/domain/mod.rs` `SUPPORTED_AGENT_IDS` to include `antigravity-cli` and add the `Antigravity` payload variant
- [x] 5.2 Add capability declarations to the payload kinds so credential support and endpoint-override support are data, not agent-id conditionals (`supports_credential` in Rust; `payloadSupportsCredential`/`payloadSupportsEndpointOverride` in TypeScript)
- [x] 5.3 Add the `antigravity-cli` branch to `live_config.rs` `primary_path` resolving `~/.gemini/antigravity-cli/settings.json`
- [x] 5.4 Implement `antigravity_fragment` and `project_antigravity`, preserving unmodelled keys
- [x] 5.5 Reject credential submission for credential-free kinds before any configuration file is touched
- [x] 5.6 Add an official preset for `antigravity-cli` that declares no base URL and no authentication strategy — **the preset test's "≥8 presets per Agent" invariant assumed every config-managed CLI is a relay target**; it now expects 1 for Antigravity and 8+ for endpoint-capable Agents
- [x] 5.7 Add unit tests for apply, drift, malformed-document reporting, and unmodelled-key round-tripping
- [x] 5.8 Derive the write-lock map from `SUPPORTED_AGENT_IDS` instead of a hand-maintained copy — the copy had already drifted, which silently left the new Agent unlockable and failed apply with "unsupported CLI agent id"

## 6. CLI launch parameters

- [x] 6.1 Add the `antigravity-cli` catalog to `src-tauri/src/contexts/tooling/cli_parameters.rs` with `--model`, `--effort`, `--agent`, and `--sandbox` — **`--dangerously-skip-permissions` deliberately excluded**: the catalog asserts no managed CLI exposes a flag containing "dangerously", and the other four all satisfy it by exposing the tool's graduated mode instead. Antigravity's graduated modes are settings keys, not flags, so the permissive posture belongs to the configuration profile (task group 5)
- [x] 6.2 Mirror the catalog in `src/services/cli-parameter-catalog.ts` and extend `managedCliAgentIds` in **both** `src/contracts/agent.ts` and `src/types/agent.ts` (two parallel declarations exist)
- [x] 6.3 Assert that `-p`, `--output-format`, `--conversation`, and any "dangerously" flag are absent from the catalog, on both the Rust and TypeScript sides
- [x] 6.4 Add catalog and preview unit tests on both the Rust and TypeScript sides
- [x] 6.5 Add the 17 `cliParameters.antigravity-cli.*` strings to all five locale files (pulled forward from 9.5 — the catalog copy test requires them)

## 7. Policy template projection

- [x] 7.1 Project `readonly` to `--sandbox` and `standard`/`trusted`/`yolo` to no sandbox flag for `antigravity-cli`, with all four combinations pinned in `fixtures/policy-template-overrides.json`
- [x] 7.2 Assert that no policy template ever emits `--dangerously-skip-permissions` — covered by the existing `policy_template_overrides_never_introduce_a_dangerous_flag`, which now iterates the widened `POLICY_TEMPLATE_GOVERNED_AGENT_IDS`
- [x] 7.3 Fail the interactive launch when the policy template cannot be resolved for `antigravity-cli` — inherited by adding it to `POLICY_TEMPLATE_GOVERNED_AGENT_IDS`, which is what `cli_profile.rs` gates the required lookup on
- [x] 7.4 Add `antigravity-cli` to the agent policy settings list as a fifth managed CLI principal

## 8. Model discovery, usage, and remaining CLI surfaces

- [x] 8.1 Read the active model from `~/.gemini/antigravity-cli/settings.json` for session chat configuration defaults, falling back on absent, keyless, or malformed documents
- [x] 8.2 Ingest `result.usage` into reported-token persistence, folding thinking tokens into output — completed by task group 4: the parser emits `Completed(Some(usage))` and `process_adapter.rs:644` persists whatever the parser reports through `normalize_provider_usage`, which is a plain field copy with no per-agent branch
- [x] 8.3 Add `antigravity-cli` to prompt-hook CLI bindings and skill mount carriers (the shared memory pool needs no change — it is already host-level rather than per-agent)
- [x] 8.5 Close the gaps a full four-CLI sweep surfaced: `ChatAgent::parse` **rejected `antigravity-cli` outright** (`UnsupportedChatAgent`, so session chat configuration would fail), the model-family maps resolved it to `Unknown` instead of `Google` on both the Rust and TypeScript sides, and the session sidebar's `sessionAgentFilters` could not filter its sessions
- [x] 8.6 Extend every settings string that enumerates the managed CLIs — custom instructions, the tool-assisted memory toggle, and the agent-policy description (that last one was already stale, naming only Claude Code) — across all five locales
- [x] 8.4 Add `antigravity-cli` to CLI chat invocation coverage for custom-instruction and memory injection ordering — no change needed once the invocation builder exists: `service.rs:1697` gates assembly on `agent.launch().kind_str() == "cli"` rather than on an Agent-id list, and the seeded launch kind is `cli`

## 9. Frontend surfaces

- [x] 9.1 Add the `antigravity` payload interface and extend `cliConfigAgentIds` in `src/types/cli-agent-config.ts`
- [x] 9.2 Render the configuration dialog from the payload kind's capability declarations so no credential field or validation action appears for `antigravity-cli`
- [x] 9.3 Add visual identity entries in `src/lib/agent-visual-identity.ts`, the `--agent-antigravity` colour token across all three themes in `styles.css`, and an `agent-brand-icon.tsx` case — **a generic `Orbit` mark, not a hand-drawn approximation of Google's trademarked logo**; swap for the official asset when one is redistributable
- [x] 9.4 Add the agent to `src/services/mock-agent-data.ts` and the Web/mock client so the Web adapter stays interface-identical with the Tauri adapter
- [x] 9.5 Add strings to all five locale files (`en`, `zh-CN`, `zh-TW`, `ja`, `ko`) — 26 keys each
- [x] 9.6 Add the agent to the create-session dialog ordering; the session sidebar and session cards render through `getAgentVisualIdentity`, so they follow automatically

## 10. Verification

- [x] 10.1 `npm run lint:ci`
- [x] 10.2 `npm run test` — 151 files / 671 tests
- [x] 10.3 `npm run build`
- [x] 10.4 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
- [x] 10.5 `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- [x] 10.6 `cargo test --manifest-path src-tauri/Cargo.toml` — 1716 passed, 0 failed (earlier `mcp::relay` and `platform::process` failures were load/environment flakes and cleared on a quiet run)
- [x] 10.7 `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 10.8 `openspec validate --specs --strict` and `openspec validate add-antigravity-cli --strict`
- [x] 10.9 `npx playwright test` — 82 passed. **Must be run with `PLAYWRIGHT_PORT` pinned to a free port**: the config defaults to 5174 with `reuseExistingServer: true`, and another worktree's dev server was listening there, which would have silently tested that checkout's code instead
- [x] 10.10 `npm run contracts:check`, `npm run test:coverage`, `npm run coverage:policy:test`, `npm run version:unit:test`, `npm run docs:check`
- [ ] 10.11 Launch the desktop app against an authenticated `agy` install and confirm an end-to-end managed chat invocation streams output and records reported usage
