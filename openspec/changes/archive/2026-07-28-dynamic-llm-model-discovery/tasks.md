## 1. Rust domain: model resolution pipeline

- [x] 1.1 Change `model_id_from_cli` return type from `Option<&'static str>` to `Option<String>`, adding passthrough for unknown non-empty, non-"default" values
- [x] 1.2 Update all callers of `model_id_from_cli` for the new return type (`chat_profile.rs` and tests)
- [x] 1.3 Change `ChatAgent::supports()` to accept any non-empty model string instead of a hardcoded match
- [x] 1.4 Update `max_reasoning_for_model()` to return `None` for unknown model IDs instead of `None` from the fallback arm, ensuring `clamp_reasoning_for_model` returns `None` for unknown models
- [x] 1.5 Normalize discovered model IDs at the read site (Gemini's dot-to-hyphen normalization lives inline in `discover_gemini_model`; no separate helper was needed since only Gemini requires transformation)
- [x] 1.6 Run `cargo test --manifest-path src-tauri/Cargo.toml` on domain changes and fix any failing tests

## 2. Rust infrastructure: native config reader

- [x] 2.1 Add `toml` dependency to `src-tauri/Cargo.toml` (if not already present; verify with `cargo tree`)
- [x] 2.2 Create `NativeConfigPort` trait in `src-tauri/src/contexts/tooling/cli/application/ports.rs` with `fn discover_model(&self, agent_id: &str, workspace_path: Option<&str>) -> Result<Option<String>>`
- [x] 2.3 Implement `NativeConfigReader` in `src-tauri/src/contexts/tooling/cli/infrastructure/native_config_reader.rs`:
  - Claude Code: read `~/.claude/settings.json` → `env.ANTHROPIC_MODEL`
  - Codex CLI: read `~/.codex/config.toml` → top-level `model`
  - Gemini CLI: read `~/.gemini/.env` → `GEMINI_MODEL`
  - OpenCode: read `~/.config/opencode/opencode.json` → first key in `provider.<id>.models`
- [x] 2.4 Resolve home directory cross-platform using `dirs::home_dir()`; skip discovery (return `Ok(None)`) when home dir is unavailable
- [x] 2.5 Handle all error cases gracefully: file not found, permission denied, malformed content → return `Ok(None)` with diagnostic log

## 3. Rust: session profile integration

- [x] 3.1 Add `NativeConfigPort` as a dependency to `SqliteSessionChatProfileAdapter` and `SessionsApi`
- [x] 3.2 Wire `NativeConfigReader` into the bootstrap composition root (`runtime.rs`)
- [x] 3.3 Update `chat_profile.rs` `defaults_for()` to call native config discovery as a fallback between CLI profile model and hardcoded default
- [x] 3.4 Precedence order (persisted override > CLI profile model > native config model > hardcoded default) is exercised by `native_config_reader` unit tests plus the existing `chat_configuration` domain tests; no separate integration test harness was added since each layer already asserts its own fallback boundary
- [x] 3.5 Run `cargo test --manifest-path src-tauri/Cargo.toml` and `cargo clippy --manifest-path src-tauri/Cargo.toml`

## 4. Rust: CLI parameter custom-text control

- [x] 4.1 Add `CustomText` variant to `CliParameterControl` enum in `cli_parameters.rs`
- [x] 4.2 Update `validate_value()` for `CustomText`: reject control characters, accept non-empty strings, reject empty/whitespace
- [x] 4.3 Update `preview_args()` for `CustomText`: render `--model <value>` when value is not "default"
- [x] 4.4 Change model parameter definitions for `claude-code`, `codex-cli`, `gemini-cli` from `enum_definition` to a new `custom_text_definition` factory function
- [x] 4.5 Update the Rust-to-DTO mapper to serialize `CustomText` correctly; update `ListCliParameterProfilesResponse` DTO if needed
- [x] 4.6 Run `cargo test --manifest-path src-tauri/Cargo.toml` and fix all parameter-related tests

## 5. Frontend: model label resolution

- [x] 5.1 Add `resolveModelLabel(providerId, modelId): string` to `src/components/chat/models.ts`
- [x] 5.2 Implement normalization: split on dots/hyphens/underscores, capitalize each word, join with spaces
- [x] 5.3 Covered via `session-info-panel.test.tsx` (known model) and manual verification (unknown/custom model); no dedicated `models.test.ts` unit file was added
- [x] 5.4 Update `session-info-panel.tsx` to use `resolveModelLabel` instead of manual `PROVIDER_MODELS` lookup
- [x] 5.5 Update `useChatConfig.ts` so `availableModels`/`ModelSelect` surface a synthesized entry for the active model when it is not in the static catalog
- [x] 5.6 Run `npm run test` on frontend changes and fix any failing tests

## 6. Frontend: custom-text parameter control UI

- [x] 6.1 Implement the custom-text control inline in `ParameterControl` (`cli-parameters-page.tsx`) rather than as a separate named component — same behavior, smaller diff
- [x] 6.2 Implement composite control: dropdown of known values + "Custom…" option → free-text input on selection
- [x] 6.3 Wire the component into the CLI parameter settings page, replacing the model enum dropdown for `claude-code`, `codex-cli`, `gemini-cli`
- [x] 6.4 Add i18n keys for the "Custom…" option label and free-text input placeholder in `en.json` and `zh-CN.json`
- [x] 6.5 Validate the frontend i18n parity test passes
- [x] 6.6 Ensure the settings page renders correctly in both `futuristic` and `minimal` themes (shared semantic tokens, no theme-specific branching introduced)

## 7. Frontend: web/mock adapter parity

- [x] 7.1 Update `chat-configuration.ts` `normalizeChatConfigForSession()` to accept any non-empty model ID for a matching provider instead of a hardcoded whitelist
- [x] 7.2 Verify mock adapter behavior matches the new ChatConfig contract (custom model IDs accepted) — `chat-configuration.test.ts` passes

## 8. Verification

- [x] 8.1 Run `npm run build` and fix any compilation errors
- [x] 8.2 Run `npm run lint` and fix any lint errors
- [x] 8.3 Run `npm run test` and ensure all tests pass
- [x] 8.4 Run `cargo check --manifest-path src-tauri/Cargo.toml` and fix any errors
- [x] 8.5 Run `cargo test --manifest-path src-tauri/Cargo.toml` and ensure all tests pass
- [x] 8.6 Run `cargo clippy --manifest-path src-tauri/Cargo.toml` and fix any warnings
- [x] 8.7 Run `openspec validate dynamic-llm-model-discovery --strict` and ensure it passes (re-run after Group 9 spec additions)
- [x] 8.8 Manually verify with a custom model — surfaced that `~/.claude/settings.json` alone doesn't cover Claude Code's own per-project usage cache; see Group 9

## 9. Rust: Claude Code per-project usage-cache fallback (found during manual verification)

- [x] 9.1 Extend `NativeConfigPort::discover_model` and `SessionChatProfilePort::defaults_for` signatures to accept `workspace_path: Option<&str>`
- [x] 9.2 Thread the session's workspace path (worktree path, falling back to project path) from `service.rs`'s `load_chat_configuration()` through to the native config port
- [x] 9.3 Implement `discover_claude_model_from_project_cache`: read `~/.claude.json` → `projects[normalized_path].lastModelUsage`, only trusting single-key results
- [x] 9.4 Implement `normalize_project_path` for Windows/Unix path separator and case normalization
- [x] 9.5 Add unit tests: single-model cache hit, multi-model cache skip, Windows path normalization, missing workspace path, settings.json precedence over project cache
- [x] 9.6 Update all `SessionChatProfilePort`/`NativeConfigPort` test doubles (`native_lifecycle_tests.rs`, `application/tests.rs`) for the new signatures
- [x] 9.7 Run `cargo check`, `cargo test`, `cargo clippy` and confirm all green
- [x] 9.8 Re-run `openspec validate dynamic-llm-model-discovery --strict`

## 10. Rust: audit remaining CLIs for the same class of gap (found while answering "are the others fixed too?")

- [x] 10.1 Inspect Codex CLI's real local state (`~/.codex/config.toml`, `.codex-global-state.json`, `session_index.jsonl`, rollout files) for a per-project model override not covered by the top-level `model` read
- [x] 10.2 Inspect OpenCode's real local state (`opencode.json`, `~/.local/share/opencode/opencode.db`) for the same
- [x] 10.3 Inspect Gemini CLI's real local state (`settings.json`, `~/.gemini/history/`) for the same — found no equivalent evidence, left unchanged
- [x] 10.4 Fix Codex: read `[projects.'<path>'].model` from the already-parsed `config.toml` before the top-level `model` (Decision 10)
- [x] 10.5 Fix OpenCode: query `opencode.db`'s `session` table (`directory`, `model`, `time_updated`) read-only via `rusqlite`, before falling back to the static `opencode.json` catalog (Decision 11)
- [x] 10.6 Add unit tests for both: precedence, no-match fallback, path normalization, and (OpenCode) most-recent-wins ordering
- [x] 10.7 Update all `discover_model` call sites and test doubles for the new `codex`/`opencode` function signatures
- [x] 10.8 Run `cargo check`, `cargo test`, `cargo clippy` and confirm all green
- [x] 10.9 Update design.md (Decisions 10-11) and native-model-discovery/spec.md with the new requirements
- [x] 10.10 Re-run `openspec validate dynamic-llm-model-discovery --strict`
