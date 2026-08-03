# Verification Record

## Migration smoke matrix

Automated migration and repository fixtures exercise the required database states:

| Database state | Coverage | Result |
| --- | --- | --- |
| Clean database | `seeds_onepiece_and_existing_cli_agents_idempotently` and native lifecycle initialization | Pass |
| Pre-OnePiece database | migration fixture upgrade through schema version 36 | Pass |
| Configured user API Agent | API registration round-trip and user-origin preservation fixtures | Pass |
| Existing API row with id `onepiece` | `adopts_an_existing_api_onepiece_without_replacing_configuration` | Pass |
| Incompatible non-API `onepiece` row | `rejects_a_non_api_onepiece_collision_without_modifying_it` | Pass |

The reset repository test additionally proves that sessions, Skills, memories, usage records, and Loop references survive provider reset on stable id `onepiece`.

## Logging and secret audit

- New native diagnostics use `AgentLoggingPort`, which is backed by the unified logging service in desktop composition.
- Credential inspection failures log only a safe classification plus internal Agent id; the credential-store error text is deliberately omitted.
- Core-instruction tracing records only the semantic version. Core Markdown, Skill bodies, memory bodies, request headers, credentials, and raw provider payloads are not written to the new log entries.
- Skill-budget diagnostics contain only safe Skill names and character counts.
- OnePiece read/save/reset responses contain `credentialPresent` only. Raw API keys remain input-only and are not stored in frontend Web/mock state.
- React components perform no direct filesystem logging and all Tauri calls remain inside the service adapter.

## Automated integration

- Web/mock Playwright covers OnePiece configuration, local API-session creation, API chat rendering without an Agent Terminal, all four built-in CLI candidates, and capability-driven discovery of a user API Agent.
- Unit and integration tests cover provider/interface replacement, validation, credential compensation, reset, built-in delete protection, readiness decoration, core/Skill/memory prompt assembly, session eligibility, remote rejection, service contracts, and UI behavior.
- The OnePiece session-chat regression check confirms native chat-configuration validation accepts the stable `onepiece` identity, resolves the active native model, persists the user/assistant message pair, applies the versioned OnePiece core prompt, and reaches the configured provider runtime. The available desktop credential was rejected by the provider, so no successful real-provider completion is claimed.
- Provider-failure regression checks confirm OpenAI-compatible and Anthropic HTTP failures retain the provider diagnostic only for unified logging while chat messages receive fixed, secret-free guidance. The live DeepSeek rejection is now presented as an API-key configuration problem instead of the generic `OnePiece command failed` wrapper.

## Manual provider verification

The 2026-08-03 desktop verification used the native Tauri application, real SQLite persistence,
Windows Credential Manager, and real provider network requests. No Web/mock adapter was involved.

- **Anthropic Messages:** a catalog-owned DeepSeek Anthropic Messages Profile at
  `https://api.deepseek.com/anthropic` passed the one-token credential probe and produced a
  streamed `ANTHROPIC_STREAM_OK` response. The response restated the OnePiece local coding and
  safety role, proving the versioned core instructions reached the provider. This validates the
  real Anthropic Messages wire protocol with a DeepSeek credential; it does not claim use of an
  Anthropic-official account. On 2026-08-04 the user explicitly accepted DeepSeek provider
  verification in place of an Anthropic-official account because no Anthropic API key was
  available. The saved DeepSeek credential was revalidated through the real Tauri command with
  HTTP 200 in 201 ms, and a new desktop generation exposed `DEEPSEEK_95_REAL_OK` while generation
  was still active before completing with the OnePiece local coding and secret-safety role.
- **Approval-gated tools:** with trust disabled, a read-only `git status --short` shell call
  entered `awaiting_approval`, executed only after explicit approval, and completed normally.
- **Skill injection:** the real check first exposed that API Skill binding incorrectly validated
  against the CLI mount catalog. After changing the boundary to query registered API Agents, the
  TDD Skill bound to built-in `onepiece` and the provider returned its exact four-step discipline:
  identify behavior, add/update tests, implement minimally, then run verification. The temporary
  binding was removed afterward.
- **Memory and compaction:** the `remember` tool saved a unique marker that a separate local
  session recalled, after which the marker was deleted. A 61,000-character low-information turn
  triggered the `Conversation compacted` rich block, and a following request still identified
  `OnePiece / 本地编码代理`, proving the core prompt survived compaction.
- **Reset:** an isolated temporary AppData instance was configured, generated
  `BEFORE_RESET_OK`, then reset to provider `VaneHub`, no model, no credential, zero Profiles,
  and registry state `unavailable`. A previously created API session rejected the next message
  without a provider response. The first live pass exposed a generic `OnePiece command failed`
  chat error. After mapping only known local OnePiece configuration failures to a fixed safe
  message, the same reset database and old session were reopened in the real desktop app and a
  new message failed with actionable endpoint/model/API-key guidance while the detailed missing-
  model diagnostic remained in unified logs. The original user AppData was then restarted and
  confirmed to contain only its original active DeepSeek OpenAI Profile with its original stable
  Profile id.
- **OpenAI-compatible:** the original DeepSeek OpenAI Chat Completions Profile at
  `https://api.deepseek.com/v1` streamed `ONEPIECE_STREAM_OK` in about 1.8 seconds. A reasoning
  request persisted 267 characters of `thinking_content` before the final `391`. The UI exposed
  only reviewed fixed endpoints, provider activation preserved Agent id `onepiece`, and the
  existing valid credential was submitted through the replacement path, probed successfully,
  and saved without changing Profile identity or runtime configuration.
- **Loop trust:** with `auto_approve_tools = false`, a OnePiece worker/verifier definition was
  rejected before persistence with the tool-use trust error. Temporarily enabling trust allowed
  the same stable Agent to persist as both Worker and Verifier. The definition was deleted, no run
  was started, and trust was restored to false.

The live checks also found and fixed three ordering/boundary defects: built-in API Agents were not
recognized by Skill binding; switching from a later-inserted active Profile back to an earlier
Profile could violate the partial unique active index; and known post-reset configuration failures
fell back to the generic command error. Focused Rust regression tests cover all three while keeping
raw provider diagnostics out of chat messages.

## Final quality gates

The post-fix automated verification passed, and the accepted DeepSeek manual verification brought
OpenSpec tasks to 135/135:

- `npm run lint`, `npm run test` (115 files, 439 tests), and `npm run build`: pass.
- `cargo test --manifest-path src-tauri/Cargo.toml` (1,056 unit tests plus 11 architecture tests),
  `cargo check`, and `cargo clippy --all-targets -- -D warnings`: pass.
- `npx playwright test tests/e2e/onepiece-agent.spec.ts`: 3/3 pass.
- `openspec validate add-onepiece-native-agent --strict`: pass.
- `openspec validate --specs --strict`: 82/82 main specs pass.
- `git diff --check`: pass. A targeted credential-pattern scan found only existing explicit test
  fixtures; neither live credential used for acceptance was present in source or change artifacts.
