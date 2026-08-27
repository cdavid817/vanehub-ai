# Change: Add Unified Personalization Governance

## Why

VaneHub currently exposes one host-level custom-instructions configuration and one host-level memory pool to OnePiece and the built-in CLI wrappers. This provides basic cross-Agent continuity, but it also collapses unrelated projects, Agents, sessions, and provider paths into one implicit trust boundary.

The current implementation has several correctness and governance gaps that must be fixed before expanding personalization:

- `MemoryDirectory::scan()` is capped at 200 parsed files but is reused by destructive reset and filename-conflict detection. A reset can therefore leave older or malformed files behind, and a new memory can collide with an unseen file and overwrite it.
- A directory-relative file path is used as the memory identity. Display-name generation, persistence identity, and update semantics are coupled.
- Custom instructions and memories are applied as host-level globals. `agent_id` and workspace folder are provenance only, not access boundaries.
- OnePiece and CLI wrappers produce long-term memory through different triggers, but the UI exposes only two broad toggles and does not explain which runtime is affected.
- Automatic extraction can persist model-inferred or tool-derived content directly into the shared pool without a review boundary.
- Personalization fields are saved through the whole `AppSettings` object, so concurrent field saves or external settings events can replace unrelated state.
- Users cannot inspect the effective instructions, memory scope, exclusions, or per-Agent behavior for a concrete generation.

Industry tools have converged on layered, inspectable configuration rather than one undifferentiated global context: user and workspace settings with precedence, project-specific instructions, repository/path-scoped rules, project-isolated memory, and explicit memory inspection. VaneHub needs the same control-plane properties while preserving its unique ability to manage OnePiece and heterogeneous CLI Agents from one desktop application.

## What Changes

- Add a dedicated personalization domain and native persistence boundary instead of using generic `AppSettings` as the runtime source of truth.
- Resolve an immutable personalization policy snapshot for every VaneHub-managed generation, using stable Agent identity, workspace identity, session identity, runtime kind, and session personalization mode.
- Support deterministic global, Agent, workspace, workspace-Agent, and session precedence without hard-coding the built-in Agent list.
- Add `standard`, `project-only`, and `temporary` session personalization modes. Temporary sessions keep ordinary custom instructions but neither read nor create long-term memory.
- Preserve separate internal context-compaction behavior for OnePiece, Claude Code, Codex, OpenCode, Gemini CLI, Antigravity, and future runtimes. This change governs VaneHub-owned long-term personalization only.
- Replace path-derived memory identity with an immutable UUID/ULID identifier and atomic file persistence.
- Add global/workspace memory scopes, optional Agent audience restrictions, lifecycle status, provenance, revision, and review state.
- Change automatic OnePiece and CLI extraction to create reviewable memory candidates by default instead of directly mutating active memory.
- Separate paged user queries from unbounded internal enumeration used by migration, reset, and repair; remove the 200-file correctness hazard.
- Coordinate memory file deletion, SQLite projection deletion, derived `MEMORY.md` rebuilding, and retrieval-index revocation through one application service.
- Add optimistic concurrency for policy and memory edits.
- Redesign **Settings → AI Personalization** into Overview, Instructions, Memory, and Runtime Preview views with dynamic Agent coverage, scoped overrides, candidate review, search/filter/pagination, safe reset, and diagnostics.
- Add a session-mode selector to session creation and a persistent mode indicator in the active conversation.
- Migrate existing custom instructions, toggles, and memory files idempotently while preserving legacy behavior as the initial global default.

## Capabilities

### New Capabilities

- `unified-personalization-governance`: deterministic policy resolution, per-generation snapshots, dynamic Agent coverage, session modes, scoped persistence, safe failure behavior, and effective-context inspection.

### Modified Capabilities

- `custom-instructions`: replace one host-wide switch with inherited, scoped instructions resolved for every VaneHub-managed runtime.
- `agent-cross-session-memory`: replace the unrestricted shared pool and path identity with governed scope, stable identity, review, lifecycle, paged management, coordinated reset, and runtime-specific adapters.
- `session-management`: persist and expose the personalization mode used by each session.
- `app-settings`: migrate legacy personalization fields out of generic settings and into the dedicated personalization service.

## Behavioral Compatibility

- Existing installations retain their current custom-instruction text and enablement as the initial global personalization policy.
- Existing valid memories migrate to active global memories with their original content and provenance, so no previously available memory is silently discarded.
- Existing sessions migrate to `standard` mode.
- OnePiece continues to use VaneHub-native context compaction. CLI tools continue to own their internal compaction, native memory files, and native instruction files.
- CLIs launched directly outside VaneHub are not governed by VaneHub personalization.
- Automatic extraction changes intentionally: newly inferred memories enter the review queue by default. Explicit user memory actions may create active memories directly.

## Impact

### Rust / Tauri

- Add `src-tauri/src/contexts/personalization/` with domain, application, infrastructure, and API layers.
- Refactor current memory persistence out of `agent_runtime` behind the personalization API while retaining runtime-specific extraction and relevance-selection adapters in `agent_runtime`.
- Add the next SQLite migration for personalization policy, memory projection, migration state, and revision data.
- Replace unscoped `AgentMemoryPort::list_all` use in runtime assembly with scoped personalization resolution.
- Add typed Tauri commands for policy, preview, memory query/detail/update/review/reset, and repair.
- Update bootstrap/composition-root wiring and command registration.

### React / TypeScript

- Add typed personalization contracts and extend `AgentService`; keep Tauri and Web/mock adapters behaviorally equivalent.
- Replace the existing two-section personalization page with four progressive views and scoped editing workflows.
- Add session personalization mode controls and indicators.
- Add complete localization and accessibility coverage.

### Data and Operations

- Run an idempotent v1-to-v2 memory migration and rebuild the derived index/projection.
- Quarantine malformed legacy memory files rather than silently losing or activating them.
- Add reconciliation and reset outcome reporting.
- Add regression coverage for 201+ and 1,000-memory stores, malformed files, concurrent edits, project isolation, dynamic Agents, and temporary sessions.
