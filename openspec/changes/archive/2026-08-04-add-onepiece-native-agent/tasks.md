## 1. Resolve Product Inputs and Test Baselines

- [x] 1.1 Write and review the initial OnePiece core-instruction Markdown asset, assign its first semantic version, and add a focused assertion that the shipped content is non-empty and no longer than 8,000 Unicode characters.
- [x] 1.2 Resolve the create-session default-selection policy from `design.md` and record it there; preserve the existing default CLI selection unless the user explicitly selected a ready OnePiece previously.
- [x] 1.3 Inventory every current `agents.id` reference and API-Agent registry/configuration call site so migration, adoption, delete protection, and contract updates cover Sessions, Skills, memories, usage, Loops, scheduled tasks, workflow state, and tests.
- [x] 1.4 Add or identify database fixtures for a clean database, a pre-OnePiece database, a configured user API Agent, an existing API row with id `onepiece`, and an incompatible non-API `onepiece` collision.

## 2. Agent Origin Migration and Stable Identity

- [x] 2.1 Add an idempotent SQLite migration for `agents.agent_origin`, validate allowed `builtin`/`user` values, backfill the four seeded CLI Agents as built-in, and backfill existing API Agents as user-origin.
- [x] 2.2 Extend API-Agent registration persistence so every newly registered user API Agent explicitly stores `agent_origin = user`.
- [x] 2.3 Extend the native seed catalog with stable id `onepiece`, display name `OnePiece`, provider placeholder `VaneHub`, `launch_kind = api`, API mode, built-in origin, safe trust default, and the specified capability tags.
- [x] 2.4 Implement migration/seed adoption for an existing API row with id `onepiece`, preserving its configuration and all id-based references while adding missing origin, mode, and tag metadata.
- [x] 2.5 Reject an incompatible non-API `onepiece` collision before modifying the row and report the failure through the existing database/bootstrap error boundary.
- [x] 2.6 Extend `AgentDefinition`, `AgentView`, Rust command DTOs/mappers, TypeScript contracts/types, and contract-conformance tests with the minimal management-origin metadata required by lifecycle and UI behavior.
- [x] 2.7 Add migration and repository tests covering clean initialization, upgrade seeding, idempotent reopen, user-origin preservation, API collision adoption, incompatible collision rejection, and unchanged existing CLI seeds.

## 3. OnePiece Provider Configuration and Lifecycle

- [x] 3.1 Add application models and narrow ports for reading, saving, and resetting OnePiece provider configuration without exposing raw credentials in return values.
- [x] 3.2 Centralize provider/interface/model/Base URL validation so OnePiece configuration and ordinary API-Agent registration enforce the same Anthropic and OpenAI-compatible rules without duplicating validation branches.
- [x] 3.3 Implement repository operations that configure OnePiece in place, may replace its provider/interface format, preserve stable id and references, and reject calls for a missing or non-built-in/non-API target.
- [x] 3.4 Implement credential replacement compensation: preserve the prior credential state, store the replacement, persist SQLite configuration, and restore/remove the credential if persistence fails.
- [x] 3.5 Implement reset so SQLite becomes structurally unavailable first, provider fields return to the unconfigured defaults, automatic tool approval is disabled, credential removal is attempted, and identity/session/Skill/memory/usage/Loop data is preserved.
- [x] 3.6 Enforce built-in API-Agent deletion rejection in both application and repository layers while retaining existing reference-aware deletion for user-origin API Agents.
- [x] 3.7 Add Tauri commands, command DTOs, mapper functions, error mappings, and command-registry entries for get/save/reset OnePiece configuration, returning only non-secret configuration state.
- [x] 3.8 Add application, repository, command-mapper, and credential-failure tests for first configuration, provider replacement, interface replacement, validation rejection, compensation, reset, delete protection, and ordinary API-Agent lifecycle regression.

## 4. Credential-Aware Registry Readiness

- [x] 4.1 Add a credential-aware `AgentRegistryRepository` decorator that wraps structural registry reads, credential presence checks, and unified logging without contacting any provider.
- [x] 4.2 Map incomplete API configuration to `unavailable`, complete configuration without a credential to `needs-auth`, complete configuration with a credential to `available`, and credential-store inspection failure to a safe non-selectable state.
- [x] 4.3 Ensure a credential failure for one API Agent logs a redacted warning and does not prevent unrelated registry entries from being listed or retrieved.
- [x] 4.4 Wire the decorated registry into Agent Runtime list/get/select, Loop eligibility, workflow selection, and all other consumers that require authoritative availability.
- [x] 4.5 Add focused decorator tests for Anthropic and OpenAI-compatible configurations, missing credential, store failure, per-Agent degradation, no provider network access, and existing CLI availability behavior.

## 5. OnePiece Core Instructions and Prompt Assembly

- [x] 5.1 Add `AgentCoreInstructionsPort` and a native adapter that returns the compiled OnePiece asset plus version for id `onepiece` and no core instructions for other Agents.
- [x] 5.2 Extend API runtime composition and test doubles to inject the core-instructions port without introducing OnePiece-specific provider execution routing.
- [x] 5.3 Refactor system-prompt assembly into independently resolved core, Skill, and memory sections with deterministic Core → Skills → Memories ordering and provider-native placement.
- [x] 5.4 Enforce the 8,000-character per-Skill and 16,000-character aggregate Skill budgets in deterministic binding order, skip oversized/non-fitting Skills as whole items, and log only safe ids/versions/sizes.
- [x] 5.5 Preserve core and memory sections when Skill lookup fails, preserve core and Skill sections when memory lookup fails, and retain ordinary API-Agent no-system-prompt behavior when every source is empty.
- [x] 5.6 Attach the OnePiece core version to safe prompt/generation tracing while excluding core text, Skill bodies, memory bodies, credentials, and raw provider payloads from logs.
- [x] 5.7 Add unit tests for section ordering, empty optional sections, core-only OnePiece generation, ordinary API Agent behavior, both lookup-failure paths, Skill budgets, provider wire formats, tool-loop round trips, and compaction preservation.

## 6. Native Session Eligibility and Workspace Rules

- [x] 6.1 Split Agent validation from `SessionCreationContextPort` into a narrow sessions-owned eligibility port and adapter backed by the same decorated Agent registry.
- [x] 6.2 Replace the native `browser | native-desktop | cli` string allowlist with shared interaction-mode parsing and the selected Agent's declared modes/selectability, including `api`.
- [x] 6.3 Wire the eligibility adapter without creating an Agent Runtime ↔ Sessions service-construction cycle, constructing credential/registry infrastructure before both application services.
- [x] 6.4 Reject non-ready API Agents, unknown ids, and undeclared interaction modes before session persistence or provider contact.
- [x] 6.5 Reject OnePiece combined with a remote workspace while allowing local folders and local Git worktrees; preserve all existing CLI remote/local behavior.
- [x] 6.6 Add Sessions application/infrastructure tests for ready OnePiece creation, unavailable/needs-auth rejection, mode mismatch, unknown id, local worktree success, remote rejection, and existing four-CLI regression.

## 7. Frontend Service Contracts and Runtime Adapters

- [x] 7.1 Add non-secret OnePiece configuration/readiness types and get/save/reset methods to `AgentService`, `src/types`, and `src/contracts` without using `any` or weakening strict type equality.
- [x] 7.2 Implement thin Tauri client wrappers for the OnePiece commands and keep every `invoke()` call inside `tauri-agent-client.ts`.
- [x] 7.3 Seed OnePiece in Web/mock state and implement deterministic get/save/reset behavior without retaining the submitted raw mock API key beyond a credential-present flag.
- [x] 7.4 Make Web/mock `createSession` enforce declared mode, readiness, and OnePiece local-workspace restrictions consistently with the native boundary.
- [x] 7.5 Add frontend contract-conformance, Tauri wrapper, and Web client tests for unconfigured/configured/reset transitions, secret omission, built-in delete rejection, API session creation, and CLI regressions.

## 8. OnePiece Settings and Create-Session UI

- [x] 8.1 Add OnePiece visual identity to the shared stable-id icon/tone mapping and verify registry, settings, create-session, session-list, and session-detail surfaces derive it without session icon persistence.
- [x] 8.2 Add an OnePiece settings panel/dialog that displays readiness, edits provider/interface/model/Base URL, accepts an optional replacement credential, saves through `AgentService`, and never reads a stored raw key.
- [x] 8.3 Add a confirmed OnePiece reset action, replace its API-Agent delete action with reset guidance, and leave user-origin API-Agent edit/delete UI unchanged.
- [x] 8.4 Replace `preferredAgentIds` eligibility with a tested pure selector over registry mode, origin, and availability metadata; include CLI/API candidates and exclude browser/native-desktop-only entries.
- [x] 8.5 Group candidates as VaneHub native, built-in CLI, and custom API while keeping stable ids untouched and applying the resolved default-selection policy.
- [x] 8.6 Show non-ready OnePiece disabled with its localized readiness reason and a configuration navigation action; selecting ready OnePiece SHALL derive `interactionMode = api`.
- [x] 8.7 Disable remote workspace selection for OnePiece with localized guidance while preserving local project and worktree controls and existing CLI behavior.
- [x] 8.8 Update create-session copy that currently says “CLI” where the flow now supports both CLI and API Agents.
- [x] 8.9 Add complete zh-CN/en/zh-TW/ja/ko resource keys with parity tests for OnePiece identity, setup, readiness, reset, session grouping, and local-only guidance.
- [x] 8.10 Add focused React tests for OnePiece setup/reset, candidate discovery and grouping, unavailable navigation, API mode selection, local/remote rules, visual identity, and existing CLI selection regressions.
- [x] 8.11 Establish the CLI-aligned OnePiece toolbar, readiness/status summary, provider-card area, and unconfigured empty state used by the provider configuration surface.
- [x] 8.12 Add an application-owned OnePiece add/edit API-provider dialog that preserves validation, never repopulates stored credentials, and remains usable at narrow widths.
- [x] 8.13 Add zh-CN/en/zh-TW/ja/ko copy and focused React tests for “Add API provider”, provider-card metadata, dialog add/edit behavior, optional credential replacement, and reset.

## 9. Integration, Security, and Manual Verification

- [x] 9.1 Add Playwright coverage in Web/mock mode for configuring OnePiece, selecting it in the create-session dialog, creating a local API session, and confirming no Agent Terminal is offered.
- [x] 9.2 Add Playwright regression coverage showing the four CLI Agents remain selectable and user-created API Agents now appear through capability-driven discovery.
- [x] 9.3 Audit every new diagnostic and operation log against `openspec/specs/unified-log-management/spec.md`; verify credentials, core/Skill/memory bodies, headers, and raw provider payloads are redacted or omitted before persistence.
- [x] 9.4 Run migration smoke tests against clean, pre-OnePiece, configured API-Agent, adopted `onepiece`, and incompatible-collision databases and record results in the change artifacts.
- [x] 9.5 Manually run the Tauri desktop app with a real DeepSeek credential over its Anthropic Messages-compatible endpoint: configure OnePiece, create a local session, verify streaming, core-instruction influence, approval-gated tools, Skill injection, memory, compaction, reset, and post-reset session rejection.
- [x] 9.6 Manually repeat provider generation with a real OpenAI-compatible endpoint and verify Base URL validation, provider replacement on stable id `onepiece`, streaming/reasoning events, and credential rotation.
- [x] 9.7 Manually verify a trusted OnePiece can serve as an eligible API Loop worker/verifier while untrusted OnePiece remains rejected according to existing Loop policy.
- [x] 9.8 Add and run Playwright coverage for the CLI-aligned OnePiece provider add/edit surface and narrow-viewport behavior in Web/mock mode.

## 10. Final Quality Gates

- [x] 10.1 Run `npm run lint` and resolve all frontend lint failures.
- [x] 10.2 Run `npm run test` and resolve all frontend unit/integration failures.
- [x] 10.3 Run `npm run build` and verify the frontend chunk-budget check passes.
- [x] 10.4 Run `cargo test --manifest-path src-tauri/Cargo.toml` with interfering proxy environment variables unset and resolve all Rust test failures.
- [x] 10.5 Run `cargo check --manifest-path src-tauri/Cargo.toml` and resolve all compile errors.
- [x] 10.6 Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` and resolve all warnings without broad suppressions.
- [x] 10.7 Run the affected Playwright suites with interfering proxy environment variables unset and record pass counts.
- [x] 10.8 Run `openspec validate add-onepiece-native-agent --strict` and `openspec validate --specs --strict` and resolve every validation error.
- [x] 10.9 Review the final diff against proposal, design, and all five delta specs; confirm desktop/Web parity, no direct component `invoke()`, no plaintext secrets, and no unintended behavior changes for existing Agents.
- [x] 10.10 Re-run frontend lint, unit/integration tests, production build, affected Playwright suites, and strict OpenSpec validation after the OnePiece provider-surface redesign.
- [x] 10.11 Re-run `cargo test` and `cargo check` for `src-tauri/Cargo.toml` to confirm the frontend-only redesign preserves native integration.

## 11. Multiple OnePiece Provider Profiles

- [x] 11.1 Add an idempotent SQLite migration for non-secret `onepiece_provider_profiles`, enforce at most one active Profile, and convert a complete legacy OnePiece binding into a deterministic active Profile without changing stable Agent id or references.
- [x] 11.2 Add repository models and operations to list, save, activate, delete, and remove all OnePiece Profiles while projecting the active Profile onto the existing OnePiece Agent runtime fields.
- [x] 11.3 Add Profile-scoped credential storage and compensated activation/edit/delete flows that preserve the current active provider on failure, lazily adopt the legacy active credential, and never persist or log secrets.
- [x] 11.4 Extend Agent Runtime application models, ports, and service behavior for named Profiles, first-Profile auto-activation, explicit later activation, active deletion without implicit fallback, and built-in identity preservation.
- [x] 11.5 Replace or extend OnePiece Tauri DTOs, mappers, commands, and command registration with list/save/activate/delete Profile operations while retaining remove-all compatibility.

## 12. Frontend Profile Management

- [x] 12.1 Extend strict TypeScript contracts and `AgentService` with non-secret OnePiece Profile overview/save/activate/delete operations, and implement matching Tauri adapter wrappers without component-level `invoke()` calls.
- [x] 12.2 Implement Web/mock multiple-Profile state, per-Profile credential-presence flags, first-Profile activation, explicit switching, deletion semantics, and runtime readiness parity without retaining submitted secrets.
- [x] 12.3 Redesign the OnePiece configuration panel/dialog to match CLI Profile management: always-visible add action, multiple cards, active emphasis, edit/delete/apply controls, and application-owned confirmation dialogs.
- [x] 12.4 Add complete zh-CN/en/zh-TW/ja/ko copy for Profile name, active/inactive state, activation, deletion, empty/filter states, and confirmation/results while preserving locale parity.
- [x] 12.5 Add focused Rust, contract, adapter, Web, React, migration, and Playwright tests for multiple providers, credential isolation, activation, active deletion, legacy migration, narrow layouts, and existing CLI/session regressions.

## 13. Multiple-Profile Quality Gates

- [x] 13.1 Run focused OnePiece frontend, Web adapter, Tauri wrapper, migration/repository, and Playwright tests.
- [x] 13.2 Run `npm run lint`, `npm run test`, and `npm run build`.
- [x] 13.3 Run `cargo test`, `cargo check`, and `cargo clippy --all-targets -- -D warnings` for `src-tauri/Cargo.toml`.
- [x] 13.4 Run `openspec validate add-onepiece-native-agent --strict` and `openspec validate --specs --strict`.
- [x] 13.5 Review the final diff for desktop/Web parity, active-Profile atomicity, secret redaction, stable OnePiece identity, and no reintroduction of Agent Management UI.

## 14. Catalog-Backed OnePiece Providers

- [x] 14.1 Add a versioned OnePiece provider catalog aligned with compatible CLI provider choices, expose it through Agent Runtime models/service and a Tauri command, and keep provider identity/interface/Base URL owned by the catalog.
- [x] 14.2 Add additive Profile source-preset id/version persistence and migration behavior that associates exact known legacy configurations while preserving unmatched legacy Profiles without enabling new custom providers.
- [x] 14.3 Replace OnePiece Profile save input provider/interface/Base URL fields with a required preset id, resolve and validate catalog fields at the Rust application boundary, preserve provider immutability on edit, and retain compensated credential behavior.
- [x] 14.4 Extend strict TypeScript contracts, `AgentService`, Tauri wrappers, Web/mock state, and contract tests with provider-catalog listing and catalog-id Profile saves, without retaining secrets or accepting arbitrary endpoints.
- [x] 14.5 Replace the OnePiece free-text provider/interface/Base URL form with the CLI-style searchable official/common provider catalog; allow Profile name, model, and credential editing only, and omit the custom-provider action.
- [x] 14.6 Update zh-CN/en/zh-TW/ja/ko copy and focused Rust/Web/React/adapter/migration/Playwright tests for catalog parity, provider selection, unknown-preset rejection, immutable resolved endpoints, multiple vendors, narrow layouts, and existing session behavior.
- [x] 14.7 Run frontend lint/test/build, Rust test/check/clippy, affected Playwright suites, strict change/spec validation, and final diff review confirming no manual OnePiece provider or Base URL entry remains.

## 15. Expanded Provider Catalog and Brand Icons

- [x] 15.1 Replace the OpenCode-derived Web catalog and duplicated native entries with a reviewed versioned OnePiece catalog containing only existing Anthropic/OpenAI-chat-compatible fixed-host providers, including immutable runtime endpoint, icon key, safe help links, default/fallback models, and discovery strategy metadata.
- [x] 15.2 Add the reviewed compatible vendor set, preserve existing preset ids and legacy matching, reject unsupported-protocol/custom-host providers, and add native/Web catalog parity and uniqueness tests.
- [x] 15.3 Add a provider-brand icon component plus locally bundled SVG assets with initials fallback, dark/light-safe rendering, and a provenance/license inventory; render icons in catalog choices and saved Profile cards.
- [x] 15.4 Update zh-CN/en/zh-TW/ja/ko copy and focused React/Playwright coverage for expanded provider search, icon rendering, fallback rendering, links, and narrow layouts.

## 16. Credential-Aware Model Discovery

- [x] 16.1 Extend strict TypeScript/Rust contracts and `AgentService` with preset/Profile/transient-credential model discovery requests and non-secret model-option/result/warning responses.
- [x] 16.2 Add a native model-discovery port and proxy-aware no-redirect HTTP adapter for Anthropic, OpenAI-compatible, and catalog-only strategies with bounded timeout/body/model counts, response validation, deduplication, known non-chat filtering, fallback merge, and redacted unified logging.
- [x] 16.3 Resolve new-Profile transient credentials and existing Profile-scoped credentials at the application boundary without persisting or returning them; reject unknown presets/Profile mismatches without provider contact.
- [x] 16.4 Implement deterministic Web/mock model discovery from catalog fallbacks without network access or credential retention, and keep Tauri/Web service contracts in parity.
- [x] 16.5 Replace the OnePiece model text input with a searchable selector that loads after credential entry or from an existing Profile, supports retry/loading/warning/empty states, preserves an absent historical selection, and never silently changes the model.
- [x] 16.6 Add focused Rust, adapter, contract, Web, React, unified-log, and Playwright tests covering both response shapes, auth headers, no redirects, filtering/deduplication/fallback, stored/transient credentials, secret omission, stale results, failure recovery, and legacy selections.

## 17. Expanded Catalog Quality Gates

- [x] 17.1 Run focused OnePiece frontend, Web adapter, Tauri wrapper, native model-discovery, catalog parity, icon, and affected Playwright tests.
- [x] 17.2 Run `npm run lint`, `npm run test`, and `npm run build`.
- [x] 17.3 Run `cargo test`, `cargo check`, and `cargo clippy --all-targets -- -D warnings` for `src-tauri/Cargo.toml`.
- [x] 17.4 Run `openspec validate add-onepiece-native-agent --strict` and `openspec validate --specs --strict`, then review the final diff for desktop/Web parity, catalog-only endpoints, icon provenance, and secret redaction.

## 18. Shared Multi-Endpoint Provider Directory

- [x] 18.1 Replace the flat OnePiece provider records and duplicated CLI provider identities with one versioned 25-vendor directory whose partial endpoint map explicitly distinguishes Anthropic Messages, OpenAI Chat Completions, and OpenAI Responses records.
- [x] 18.2 Review and record every endpoint against the local Cherry Studio provider registry and CC Switch CLI presets, preserve distinct protocol URLs, document conflicts/source revisions, and reject inferred or user-authored fixed-directory endpoints.
- [x] 18.3 Extend OnePiece Rust/TypeScript contracts, Profile persistence/migration, catalog resolution, credential-aware model discovery, Tauri adapter, and Web/mock adapter with provider id plus endpoint type while preserving exact-match legacy Profiles and secret handling.
- [x] 18.4 Generate Claude Code, Codex CLI, and OpenCode preset projections from the shared directory using fail-closed Agent compatibility rules, retaining existing custom CLI Profile behavior and Agent-specific import/apply/drift semantics.
- [x] 18.5 Extract shared provider search/filter, catalog card, endpoint badge/selector, help-link, and brand-icon components and use them across the OnePiece, Claude Code, Codex CLI, and OpenCode configuration tabs without coupling their Profile forms or persistence contracts.
- [x] 18.6 Copy the matching provider SVG marks from Cherry Studio `@cherrystudio/ui`, add light/dark alias mappings and initials fallback, and record upstream revision/path, MIT license text, and trademark/non-affiliation notices.
- [x] 18.7 Add locale-parity, catalog-count/uniqueness, endpoint-evidence, native/Web parity, Agent-adapter compatibility, Profile migration, shared-component, icon, narrow-layout, and existing configuration regression tests.

## 19. Shared Directory Quality Gates

- [x] 19.1 Run focused provider-directory, OnePiece endpoint, CLI preset, adapter, React, icon, and affected Playwright tests.
- [x] 19.2 Run `npm run lint`, `npm run test`, and `npm run build`.
- [x] 19.3 Run `cargo test`, `cargo check`, and `cargo clippy --all-targets -- -D warnings` for `src-tauri/Cargo.toml`.
- [x] 19.4 Run `openspec validate add-onepiece-native-agent --strict` and `openspec validate --specs --strict`, then review the final diff for four-tab reuse, exact endpoint provenance, no synthesized URLs, desktop/Web parity, and secret redaction.
- [x] 19.5 Preserve Claude Code and Codex CLI session selection when only their optional managed SDK is missing, while continuing to reject SDK-dependent interaction modes, with matching frontend and native regression tests.
- [x] 19.6 Reorder create-session presentation to built-in CLI (Codex CLI → Claude Code → Gemini CLI → OpenCode) → OnePiece → custom API with Codex CLI as the default selectable candidate, and place OnePiece last in Agent configuration tabs with focused UI tests.
- [x] 19.7 Route OnePiece API sessions through the session chat-configuration boundary using the active native model, keep the runtime provider/model selectors read-only, and prevent API sessions from opening an Agent Terminal, with focused native/frontend regression tests.
- [x] 19.8 Preserve provider diagnostics in redacted unified logs while surfacing fixed, actionable OnePiece errors for authentication, request configuration, missing model/endpoint, rate limiting, and provider availability, with regression tests proving raw diagnostics are not exposed to chat messages.

## 20. Shared API-Key Verification

- [x] 20.1 Add a shared proxy-aware, no-redirect native provider probe for Anthropic Messages, OpenAI Chat Completions, and OpenAI Responses that sends a one-token request with a 15-second timeout, no retry, bounded error reads, safe classifications, and redacted unified logging.
- [x] 20.2 Add CLI configuration application resolution for transient drafts and saved Profile credentials across Claude Code, Codex CLI, and OpenCode, including existing-auth/official-auth unsupported results, custom Profile structural validation, scoped credential access, and no Profile mutation or application.
- [x] 20.3 Add OnePiece application resolution for transient and saved Profile credentials using compiled provider-directory endpoints, reject forged provider/endpoint/Profile/model combinations before contact, and expose matching Tauri commands and DTO mappers without returning secrets.
- [x] 20.4 Extend strict TypeScript contracts and `AgentService` with shared validation result semantics plus CLI and OnePiece request methods, implement Tauri wrappers and deterministic non-secret Web/mock behavior, and retain submitted credentials only for the duration of the call.
- [x] 20.5 Build one reusable credential-validation action/status component and integrate it into the CLI and OnePiece add/edit dialogs and saved Profile cards with loading, valid, invalid, configuration-rejected, rate-limited, unavailable, unsupported, and stale-result behavior.
- [x] 20.6 Add complete zh-CN/en/zh-TW/ja/ko copy and focused Rust, command-contract, Tauri adapter, Web/mock, React, and Playwright tests covering all four Agent tabs, all three wire protocols, transient/stored credentials, OAuth-not-applicable behavior, response classification, cancellation, narrow layouts, and secret omission.

## 21. API-Key Verification Quality Gates

- [x] 21.1 Run focused native provider-probe, CLI/OnePiece resolution, contract, adapter, Web/mock, React, and affected Playwright tests.
- [x] 21.2 Run `npm run lint`, `npm run test`, and `npm run build`.
- [x] 21.3 Run `cargo test`, `cargo check`, and `cargo clippy --all-targets -- -D warnings` for `src-tauri/Cargo.toml`.
- [x] 21.4 Run `openspec validate add-onepiece-native-agent --strict` and `openspec validate --specs --strict`, then review the diff for four-tab parity, context-owned resolution, no credential persistence, bounded provider cost, and unified-log redaction.
