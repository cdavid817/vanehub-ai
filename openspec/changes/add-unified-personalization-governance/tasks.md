## 0. Baseline, Change Validation, and Implementation Guardrails

- [x] 0.1 Read repository-root `AGENTS.md`, this proposal, design, every delta spec, and the current `custom-instructions`, `agent-cross-session-memory`, `app-settings`, `session-management`, and `agent-context-compaction` specifications before changing code.
- [x] 0.2 Run `openspec validate add-unified-personalization-governance --strict` and correct artifact-format issues before implementation.
- [x] 0.3 Record the current production call sites for `AgentMemoryPort`, `MemoryDirectory`, custom-instruction assembly, OnePiece memory selection/extraction, CLI prompt assembly/extraction, settings persistence, session creation, and Tauri/Web adapter commands.
- [x] 0.4 Add or update architecture decision comments so implementation preserves the boundary: VaneHub governs long-term personalization; each CLI still owns its internal compaction and native memory/instruction files.
- [x] 0.5 Capture baseline results for the relevant Rust, frontend, Web/mock, and desktop tests before changing persistence.

## 1. Personalization Domain and Safe Defaults

- [x] 1.1 Create `src-tauri/src/contexts/personalization/` using the repository's domain/application/infrastructure/API conventions and wire only compilation-visible skeletons first.
- [x] 1.2 Add typed stable identities for Agent, session, workspace, policy scope, and memory id without hard-coding the built-in Agent set.
- [x] 1.3 Add `PolicyToggle`, `InstructionMergeMode`, `SessionPersonalizationMode`, policy scope, policy record, policy patch, and revision-conflict domain types.
- [x] 1.4 Add `EffectivePersonalizationSnapshot`, resolved instruction segment, effective memory access, exclusion, warning, and revision-token types.
- [x] 1.5 Add memory scope, audience, status, source, provenance, sensitivity, record, candidate, query, page, reset, and reconciliation domain types.
- [x] 1.6 Implement domain validation for instruction length, memory field limits, audience size, legal scope combinations, global non-inheritance rules, and project-only workspace requirements.
- [x] 1.7 Define fail-closed safe defaults: no user instructions and no memory read/write/extraction when no validated policy can be loaded.
- [x] 1.8 Add domain unit tests for all merge modes, tri-state resolution, precedence, temporary/project-only restrictions, scope/audience eligibility, validation boundaries, and deterministic revision tokens.

## 2. SQLite Schema and Repository Ports

- [x] 2.1 Add the next migration using the existing migration registry for `personalization_policy_overrides`, `personalization_memory_projection`, `personalization_memory_candidates`, and `personalization_migration_state`.
- [x] 2.2 Add indexes for policy scope lookup, active memory scope/status lookup, source-Agent filtering, type filtering, update ordering, and candidate status ordering.
- [x] 2.3 Add application ports for policy persistence, memory repository, maintenance enumeration, memory projection, candidate persistence, retrieval-index coordination, workspace identity, and clock/id generation.
- [x] 2.4 Implement the SQLite policy repository with one row per typed scope key and expected-revision updates.
- [x] 2.5 Implement policy load/update transactions and typed conflict results without whole-`AppSettings` replacement.
- [x] 2.6 Implement the SQLite memory projection and cursor pagination without loading memory bodies for list pages.
- [x] 2.7 Implement candidate persistence and bounded rejected-candidate retention through the existing local retention policy.
- [ ] 2.8 Add migration/repository tests for fresh database creation, upgrade, constraints, indexes, rollback on failure, optimistic conflicts, paging stability, and Web/mock-equivalent fixtures.

## 3. Stable Memory Storage and Data-Loss Fixes

- [x] 3.1 Refactor the Markdown memory store so immutable UUID/ULID ids produce filenames and display names never participate in path identity.
- [x] 3.2 Split bounded `list_page` behavior from complete internal `enumerate_owned_entries`; complete enumeration must include malformed application-owned entries and must not stop at 200.
- [x] 3.3 Replace ordinary `fs::write` replacement semantics with create-new for creation and expected-revision atomic replacement for updates.
- [x] 3.4 Use same-directory temporary files, flush/sync where supported, platform-safe replacement, and directory mutation serialization following existing dependency and platform conventions.
- [x] 3.5 Preserve path traversal, symlink escape, Unicode normalization, and platform case-sensitivity protections for Windows, macOS, and Linux.
- [x] 3.6 Implement file parsing/serialization for v2 frontmatter and body with content hash and revision validation.
- [x] 3.7 Implement coordinated create/update/delete application services that update the authoritative file, SQLite projection, derived index, and retrieval index with repair-required reporting.
- [x] 3.8 Replace reset implementation with complete maintenance enumeration and a structured result containing matched, deleted files, projection rows, retrieval entries, quarantined entries, and failures.
- [x] 3.9 Add tests for 0, 1, 200, 201, and 1,000 memories; duplicate display names; create collision; stale revision; partial filesystem failure; malformed files; locked files; and idempotent repeated reset.
- [ ] 3.10 Add platform-sensitive tests for Windows case-insensitive names, macOS Unicode normalization, Linux permissions/symlinks, and remote-workspace path identity normalization where CI supports them.

## 4. Legacy Personalization and Memory Migration

- [x] 4.1 Implement a typed workspace identity resolver that prefers existing stable project/workspace ids and otherwise hashes the normalized root plus remote connection identity.
- [x] 4.2 Implement one-time migration of legacy `AppSettings` custom instruction fields and toggles into the global policy plus a OnePiece extraction override when required.
- [x] 4.3 Keep legacy personalization settings deserializable for the compatibility window but remove them from the new UI and runtime source-of-truth path.
- [x] 4.4 Implement complete v1 memory enumeration, excluding derived/temporary/lock/quarantine files by explicit rules rather than parse success.
- [x] 4.5 Convert each valid legacy memory to a v2 immutable-id file, active global scope, all-Agent audience, preserved content/provenance/timestamps, and `legacy_migration` source.
- [x] 4.6 Quarantine malformed or unsafe legacy files with diagnostic metadata instead of deleting or activating them.
- [x] 4.7 Write a migration manifest/backup map before removing a legacy source file and make every migration step idempotent after interruption.
- [x] 4.8 Rebuild the SQLite projection, `MEMORY.md`, and retrieval index from migrated active memories; set repair-required state if a derived rebuild fails.
- [x] 4.9 Add migration tests for empty, mixed-validity, 201+, 1,000-file, interrupted, duplicate-content, already-v2, and repeated-startup cases.
- [x] 4.10 Add a startup/application-state path that prevents memory use until migration is complete or a validated prior generation is available, without blocking unrelated application startup.

## 5. Policy Resolution and Effective Preview

- [x] 5.1 Implement deterministic resolution in this order: safe defaults, global, Agent, workspace, workspace-Agent, session override, hard session-mode restrictions.
- [x] 5.2 Implement instruction append/replace/disable behavior while preserving non-user core/system instructions outside personalization control.
- [x] 5.3 Implement effective memory access for read, explicit save, automatic extraction, global-memory access, workspace scope, Agent audience, status, and migration health.
- [x] 5.4 Capture one immutable snapshot at the start of each generation/seat turn; settings changes during execution must not mutate the captured snapshot.
- [x] 5.5 Cache only validated policy data, invalidate by revision/event, and use last-known-good state on transient read failure; use fail-closed behavior when none exists.
- [x] 5.6 Implement an effective-preview application service that returns safe instruction provenance, final modes, eligible/excluded memory counts, adapter behavior, warnings, and estimated context size.
- [x] 5.7 Ensure preview does not return hidden core system prompts, credentials, unredacted traces, or memory bodies unless the local user explicitly opens a permitted memory detail.
- [x] 5.8 Add resolver tests for all scope combinations, unknown/dynamic Agents, missing workspace, remote workspaces, temporary/project-only sessions, last-known-good fallback, and mid-generation policy changes.

## 6. OnePiece Runtime Integration

- [x] 6.1 Add a OnePiece personalization adapter that requests one snapshot through `PersonalizationApi` for every generation.
- [x] 6.2 Replace direct global custom-instruction reads with resolved instruction segments and preserve their deterministic location in prompt assembly.
- [x] 6.3 Replace unscoped memory listing with eligible active-memory summaries from the snapshot.
- [x] 6.4 Keep existing OnePiece relevance selection and independent memory context budget, but ensure selection receives only policy-eligible records and cannot broaden scope.
- [x] 6.5 Preserve age/staleness caveats and already-surfaced-memory exclusion for selected bodies.
- [x] 6.6 Change automatic extraction at compaction to submit create/update/archive candidates instead of directly mutating active memory.
- [x] 6.7 Change model-side memory tools to use the personalization application API; default model-originated writes to candidates unless backed by an explicit user UI memory action.
- [x] 6.8 Make temporary mode skip all long-term memory reads, tools, extraction, candidates, and retrieval writes while leaving current-session compaction intact.
- [x] 6.9 Add OnePiece integration tests for global/workspace/Agent resolution, project-only isolation, temporary mode, candidate extraction, selected-body filtering, extraction failure, and unchanged compaction behavior.

## 7. CLI Runtime Integration

- [x] 7.1 Add one shared CLI personalization adapter used by all VaneHub-managed CLI Agent implementations rather than duplicating built-in Agent ids in each path.
- [x] 7.2 Resolve a snapshot for every CLI message and prepend resolved custom instructions followed by the scoped active-memory index in the documented order relative to Prompt Hooks and the user message.
- [x] 7.3 Preserve index-only behavior unless an Agent runtime capability explicitly opts into selected-memory bodies.
- [x] 7.4 Preserve each CLI's internal compaction, native memory, and native instruction behavior; do not read or write CLI-owned files in this change.
- [x] 7.5 Change successful-turn extraction to submit candidates attributed to the actual CLI Agent, session, workspace, and source message ids.
- [x] 7.6 Keep a completed CLI response successful when extraction is unavailable or fails; surface only safe diagnostics.
- [x] 7.7 Apply per-Agent automatic-extraction policy and skip extraction in temporary mode or when no validated OnePiece extraction provider is available.
- [x] 7.8 Add contract tests covering Claude Code, Codex, OpenCode, Gemini CLI, Antigravity, and a synthetic dynamically registered CLI Agent without adding Agent-specific policy branches.
- [x] 7.9 Add tests that a CLI launched outside the VaneHub adapter remains out of scope and that VaneHub does not mutate its native configuration files.

## 8. Session Mode and Multi-Agent Propagation

- [x] 8.1 Extend session domain/storage/service types with `personalizationMode`, defaulting legacy and new unspecified sessions to `standard`.
- [ ] 8.2 Validate that `project-only` creation requires a resolvable workspace and reject invalid requests in both Tauri and Web/mock runtimes.
- [x] 8.3 Persist session mode across restart, archive/unarchive, worktree creation, and active-session switching.
- [x] 8.4 Propagate the shared session mode and workspace to every multi-Agent seat while resolving each seat's own Agent policy.
- [x] 8.5 Ensure Loop workers, scheduled runs, and sub-Agent paths that use the standard session/generation service cannot bypass snapshot resolution.
- [ ] 8.6 Add session migration, create/read/update, Web parity, multi-seat, worktree, background-run, project-only, and temporary-mode tests.

## 9. Native Commands and Frontend Service Parity

- [x] 9.1 Add typed Tauri commands for configuration, policy patch, effective preview, Agent capability listing, paged memory query/detail/create/update/review/delete, reset preview/execute, and reconciliation.
- [x] 9.2 Register commands using existing native error mapping and return typed validation/conflict/maintenance errors without leaking secrets.
- [x] 9.3 Add `src/types/personalization.ts` and extend `AgentService` with the dedicated personalization contract.
- [x] 9.4 Implement Tauri adapter mappings; React components must not invoke Tauri directly.
- [x] 9.5 Implement Web/mock behavior with deterministic policies, revisions, candidates, cursor pages, conflicts, reset tokens, session modes, and safe maintenance results.
- [ ] 9.6 Remove or deprecate `listAllMemories`, unscoped delete/reset methods, and legacy personalization settings mutations after all callers migrate.
- [x] 9.7 Add frontend contract tests proving Tauri serialization names and Web/mock return shapes remain equivalent.

## 10. AI Personalization Settings UI

- [x] 10.1 Rename the settings destination to localized **AI Personalization** and add Overview, Instructions, Memory, and Runtime Preview views using the existing settings navigation/design primitives.
- [x] 10.2 Build a page presentation model that separates query state, scope selection, drafts, pending mutations, conflict state, and maintenance state.
- [x] 10.3 Implement Overview cards and a compact dynamic Agent list showing effective source, capability, final instruction state, memory read state, and extraction state.
- [x] 10.4 Implement global, Agent, workspace, and workspace-Agent scope selection without hard-coded Agent checkboxes.
- [x] 10.5 Replace blur-only long-text saving with explicit Save/Discard, dirty state, inline 3,000-character validation, approximate token count, navigation protection, and per-scope pending state.
- [x] 10.6 Display inherited instruction text and effective source; explain append, replace, and disabled behavior before save.
- [x] 10.7 Preserve user drafts on native errors and revision conflicts and provide reload/compare actions without silent last-response-wins.
- [x] 10.8 Ensure keyboard navigation, focus management, screen-reader labels, narrow-layout behavior, and IME composition work for all instruction workflows.
- [x] 10.9 Add component tests for initial hydration, scope changes, inheritance, edit/save/discard, validation, concurrent independent saves, conflicts, external events, error recovery, accessibility, and responsive layout.

## 11. Memory Management, Review, and Diagnostics UI

- [x] 11.1 Replace the flat full-body memory list with a paged summary list and filters for search, scope, status, type, source Agent, and Agent audience.
- [x] 11.2 Keep previous page data visible during refresh, reset the cursor when filters change, and avoid per-row body requests.
- [x] 11.3 Add a detail panel for body, metadata, provenance, timestamps, scope, audience, status, revision, edit, archive/reactivate, and delete.
- [x] 11.4 Add a pending-review workflow with approve, edit-and-approve, reject, scope/audience change, merge, and conflict handling.
- [x] 11.5 Add explicit memory creation from Settings and message-menu actions for remember globally, remember for project, and forget/correct a surfaced memory.
- [x] 11.6 Add reset preview and execution dialog with exact counts, scope/status selection, typed confirmation, short-lived token, and structured result display.
- [x] 11.7 Add maintenance/health UI for migration state, malformed/quarantined entries, projection/index mismatch, last reconciliation, and rebuild action.
- [x] 11.8 Add Runtime Preview inputs and provenance/exclusion output, including clear statements that CLI internal compaction is not managed by VaneHub.
- [x] 11.9 Use normal paged document flow; add measured virtualization only if the rendered result set can exceed the established 500-row threshold.
- [x] 11.10 Add tests for paging/filtering, candidate review, edit conflicts, duplicate names, scoped reset, partial reset failure, reconciliation, preview redaction, message actions, keyboard/focus behavior, and 500+ row performance if virtualization is used.

## 12. Session Creation and Conversation UI

- [x] 12.1 Add localized Standard, Project-only, and Temporary choices near workspace selection in every session-creation surface.
- [x] 12.2 Disable Project-only without a workspace and show a specific accessible explanation.
- [x] 12.3 Add a persistent Project-only/Temporary badge in the conversation header and a concise explanation of what VaneHub does and does not retain.
- [x] 12.4 Ensure changing global settings does not mutate the mode of an existing session and policy changes apply only to later generations.
- [x] 12.5 Add React, Web/mock, Playwright, and desktop tests for creation, persistence, switching, restart, multi-seat sessions, project-only validation, and temporary behavior.

## 13. Localization, Documentation, and Cleanup

- [ ] 13.1 Add synchronized locale keys for every new label, state, validation message, conflict, migration warning, reset result, preview explanation, session mode, and accessible name in every supported locale.
- [ ] 13.2 Update the user guide to explain unified management, scope precedence, Agent coverage, session modes, candidate review, memory source/audience, reset, repair, and CLI internal-compaction boundaries.
- [ ] 13.3 Update the developer guide with the personalization context boundary, runtime adapter contract, snapshot sequence, memory file/projection authority, migration, and extension checklist for a new Agent.
- [ ] 13.4 Add a developer checklist requiring every new VaneHub-managed Agent/runtime to declare capabilities and call the personalization resolver.
- [ ] 13.5 Remove dead legacy UI components, unscoped service methods, duplicate prompt assembly, direct memory file mutations, and obsolete tests only after replacement coverage passes.
- [ ] 13.6 Search for and eliminate production `list_all`/`listAllMemories` use that bypasses a policy context, while retaining explicitly named internal maintenance enumeration.
- [ ] 13.7 Confirm no task introduces direct React-to-Tauri invocation, hard-coded Agent policy lists, or edits to CLI-owned memory/instruction files.

## 14. Required Verification

- [ ] 14.1 Run targeted Rust domain, repository, migration, OnePiece, CLI, session, command, and reconciliation tests after each corresponding task group.
- [ ] 14.2 Run targeted frontend service, settings UI, memory UI, session UI, accessibility, and Web/mock tests after each corresponding task group.
- [ ] 14.3 Run `npm run lint:ci`.
- [ ] 14.4 Run `npm run test`.
- [ ] 14.5 Run `npm run test:coverage` and confirm project thresholds remain satisfied.
- [ ] 14.6 Run `npm run build`.
- [ ] 14.7 Run `npx playwright test` for changed browser/Web/mock journeys and record the result.
- [ ] 14.8 Run the repository's desktop WebdriverIO/Tauri tests for native IPC, persistence, restart, migration, session mode, and 201+/1,000-memory reset scenarios on supported platforms.
- [ ] 14.9 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`.
- [ ] 14.10 Run `cargo check --workspace`.
- [ ] 14.11 Run `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] 14.12 Run `npm run native:panic:check`.
- [ ] 14.13 Run `cargo test --workspace`.
- [ ] 14.14 Run `openspec validate add-unified-personalization-governance --strict`.
- [ ] 14.15 Run `openspec validate --specs --strict`.
- [ ] 14.16 Review every task checkbox against actual code/tests and leave incomplete tasks unchecked; do not mark completion based only on compilation.
