# agent-cross-session-memory Delta Specification

## REMOVED Requirements

### Requirement: Memory scoping
**Reason**: This requirement defined memories as one host-level pool shared by every Agent, with producing Agent and workspace folder recorded as provenance that explicitly SHALL NOT filter injection, listing, or management. This change makes scope and audience explicit access boundaries, so the previous scenarios — including "Memories do not cross agents", which guaranteed the opposite of a boundary — no longer describe the system.

**Migration**: Replaced by "Governed memory scope and audience" below. Existing memories migrate to active global scope with an all-Agent audience, so previously visible memories remain visible after upgrade.

### Requirement: Explicit memory saving
**Reason**: Explicit saving was defined as an auto-approved model tool writing a file and index entry directly, addressed by name, with name collision meaning replacement. Creation now goes through the personalization application service with immutable ids, and a model-originated call without explicit user confirmation produces a candidate.

**Migration**: Replaced by "Governed explicit memory creation" below, which retains the tool's public name and catalog position so the declared tool prefix stays byte-identical.

### Requirement: Automatic memory extraction
**Reason**: Extraction previously applied validated create/update/delete actions directly to the memory directory, gated by two host-level toggles. Extraction now produces reviewable candidate proposals gated by the effective snapshot and never mutates active memory.

**Migration**: Replaced by "Candidate-producing automatic memory extraction" below, which preserves best-effort behavior, the single no-tool model call, and the rule that extraction never fails or materially delays compaction.

### Requirement: Memory injection into the system prompt
**Reason**: Injection was gated by one host-level toggle over an unscoped pool. Injection now operates on the captured personalization snapshot's eligible set, and index entries reference immutable ids.

**Migration**: Replaced by "Snapshot-scoped memory injection into the system prompt" below, which preserves index-not-bodies behavior, paired line and byte caps, explicit truncation disclosure, and the rule that memory content is never written into the compaction turns list.

### Requirement: Memory management
**Reason**: Management was defined as listing every stored memory with full content, deleting one at a time, and one unscoped delete-all. That contract cannot express paging, candidate review, revisioned edits, scoped reset, or reconciliation, and its list-everything shape is what coupled destructive reset to a bounded 200-file scan.

**Migration**: Replaced by "Paged governed memory management and maintenance" below, which adds bounded summary pages, detail-on-demand, expected-revision edits, scoped reset preview/execute, and complete internal enumeration for destructive work.

### Requirement: Web runtime memory toggle parity
**Reason**: Web/mock parity was defined against the two host-level toggles and their simulated events. Parity must now cover scoped policy, session modes, candidates, revision conflicts, paging, reset preview/execute, and reconciliation.

**Migration**: Replaced by "Web runtime governed memory parity" below.

### Requirement: Memory enablement toggle
**Reason**: One host-level boolean cannot express the separate read, explicit-save, automatic-extraction, and global-memory-access dimensions this change requires, nor scoped inheritance across Agent and workspace.

**Migration**: Replaced by "Scoped memory policy controls" below. The legacy toggle value migrates into the global policy's read/save/extraction defaults, preserving current behavior. A UI master control may still exist, but it edits the global policy rather than erasing narrower overrides.

### Requirement: Tool-assisted chat extraction toggle
**Reason**: This host-level sub-toggle was described only in terms of OnePiece compaction with a tool call, yet sat next to controls users read as governing every Agent. It becomes an explicit capability-aware policy dimension with clear applicability labeling.

**Migration**: Replaced by "Tool-assisted extraction policy dimension" below, which preserves the rule that this control never affects explicit saves and never governs a CLI turn.

### Requirement: Memory injection into CLI prompts
**Reason**: CLI injection was defined for a fixed agent list reading one host-level toggle over an unscoped pool. It now uses registry-derived coverage and the captured snapshot's eligible set.

**Migration**: Replaced by "Snapshot-scoped memory injection into CLI prompts" below, which preserves the independent CLI bound, the position after custom instructions and before Prompt-Hook-assembled content, and index-only behavior.

### Requirement: Automatic memory extraction for CLI-wrapped agents
**Reason**: CLI extraction wrote memories directly into the shared directory for a fixed agent list. It now submits candidates attributed to the actual registered CLI Agent, session, workspace, and source message ids.

**Migration**: Replaced by "Candidate extraction for VaneHub-managed CLI Agents" below, which preserves best-effort behavior, non-blocking delivery of the already-completed CLI result, and the rule that VaneHub never instructs a CLI to write memory files itself.

### Requirement: Deleting a memory revokes its retrieval index
**Reason**: The previous contract covered deletion and out-of-band file removal only. Coordination must now also cover archive, scoped reset, migration, and projection state across the authoritative file, SQLite projection, `MEMORY.md`, and retrieval index.

**Migration**: Replaced by "Coordinated derived-state revocation and reconciliation" below, which preserves the rule that an ineligible memory is never recalled from an orphaned derived entry.

### Requirement: Memories are addressable files
**Reason**: This requirement made the directory-relative file path the memory's identity and required that two memories never share one path. Identity becomes an immutable UUID/ULID, filenames are id-derived, and display names may be duplicated.

**Migration**: Replaced by "Memories are immutable-id addressable files" below. Legacy path-addressed files migrate to v2 id-addressed files; malformed files are quarantined rather than skipped silently.

### Requirement: Memory type taxonomy
**Reason**: The previous taxonomy silently degraded any absent or unrecognized type to untyped, including for newly created records. New v2 records must declare a recognized type, while legacy records migrate as explicitly `untyped` compatibility records that remain visible for correction.

**Migration**: Replaced by "Governed memory type taxonomy" below. The four recognized values `user`, `feedback`, `project`, and `reference` are unchanged.

### Requirement: Memory index file
**Reason**: `MEMORY.md` was reconciled against the whole directory. It must now be a derived view of active governed memories only, excluding candidates, archived records, malformed files, and quarantined files.

**Migration**: Replaced by "Derived active-memory index file" below, which preserves one pointer/hook line per included memory and the rule that the index carries no body or frontmatter.

### Requirement: Model-side memory correction
**Reason**: The memory directory was exposed to generic file tools as an auto-approved read/write scope, which lets a model bypass revision, scope, review, and index invariants. Model-proposed corrections now use typed personalization operations.

**Migration**: Replaced by "Governed model-side memory correction" below, which preserves unchanged approval behavior for paths outside governed memory storage.

### Requirement: Migration from the row store
**Reason**: The row-store migration remains necessary but is no longer sufficient: legacy path-addressed files must additionally migrate to the v2 immutable-id governed format, with quarantine, manifest, and derived-state rebuild.

**Migration**: Replaced by "Migration to governed v2 memory storage" below, which retains the existing idempotent row-to-file conversion as its first stage.

### Requirement: Relevance-selected memory bodies
**Reason**: Selection previously operated over the whole enabled pool and addressed memories by name. It must now operate only on records already eligible under the captured snapshot and address them by immutable id, so that selection cannot broaden scope.

**Migration**: Replaced by "Eligibility-filtered relevance-selected memory bodies" below, which preserves the bounded selection, the return-nothing-when-unclear instruction, and degradation to index-only injection on failure.

### Requirement: Already-surfaced memories are excluded from selection
**Reason**: Surfaced tracking keyed on memory identity alone. It must now key on immutable id plus revision, and must not override later ineligibility from policy, scope, audience, archive, or delete.

**Migration**: Replaced by "Surfaced memory id and revision exclusion" below, which preserves exclusion before the selection bound is applied and the fresh-start-per-session rule.

### Requirement: Injected memories carry age and staleness caveats
**Reason**: The previous contract covered age and staleness only. Injected bodies must additionally carry scope and provenance as data labels that never elevate memory content into higher-priority instructions, and candidates must never reach injection.

**Migration**: Replaced by "Injected memory age, staleness, and data labeling" below, which preserves human-readable elapsed age, the verify-before-asserting caveat past the threshold, and no caveat within it.

### Requirement: Web runtime parity for memory selection
**Reason**: Mock selection parity was gated by the host-level toggle. It must now mirror policy eligibility, surfaced id/revision tracking, staleness metadata, and project-only/temporary behavior.

**Migration**: Replaced by "Web runtime parity for governed memory selection" below, which preserves the rule that no real provider call is issued.

## ADDED Requirements

### Requirement: Governed memory scope and audience
The system SHALL store every memory with an explicit `global` or `workspace` scope and an optional all-Agent or selected-Agent audience. Producing Agent, producing workspace, session, message, and save source SHALL remain provenance and SHALL NOT be substituted for the explicit scope or audience. Runtime injection SHALL filter by the captured personalization snapshot before budgeting or relevance selection.

#### Scenario: Save a workspace memory
- **WHEN** an explicit user action or approved candidate saves a memory for an active workspace
- **THEN** the system SHALL persist the stable workspace key as the memory scope
- **AND** a different workspace SHALL NOT read or inject the memory

#### Scenario: Save a global memory
- **WHEN** the user explicitly chooses global scope
- **THEN** the memory MAY be eligible across workspaces according to session mode, effective global-memory access, and Agent audience

#### Scenario: Restrict a memory to selected Agents
- **WHEN** a memory audience contains selected stable Agent ids
- **THEN** only those Agents SHALL consider the memory eligible
- **AND** the source Agent SHALL NOT gain access unless included or the audience is all Agents

#### Scenario: Use project-only mode
- **WHEN** the active session uses `project-only`
- **THEN** global memories SHALL be excluded
- **AND** only memories whose workspace key matches the active workspace MAY be eligible

#### Scenario: Use temporary mode
- **WHEN** the active session uses `temporary`
- **THEN** no stored memory SHALL be read, injected, created, updated, archived, deleted by a model action, or extracted for long-term use

#### Scenario: Preserve provenance separately
- **WHEN** a memory is created by OnePiece or a CLI adapter
- **THEN** the record SHALL preserve source Agent, workspace, session, source message ids, save path, and timestamps as provenance
- **AND** changing scope or audience SHALL NOT rewrite historical provenance

### Requirement: Governed explicit memory creation
The system SHALL provide an explicit user-facing memory creation path and SHALL retain the OnePiece memory tool name and catalog position for compatibility. User-confirmed creation SHALL create an active memory through the personalization application service. A model-originated tool call without an explicit UI-backed user confirmation SHALL create a candidate by default rather than an active memory. Every create SHALL use a new immutable memory id; duplicate display names SHALL NOT overwrite an existing record.

#### Scenario: User explicitly saves a global memory
- **WHEN** the user chooses “Remember globally” for content and the effective explicit-save policy permits it
- **THEN** the system SHALL create a new active global memory with an immutable id and provenance
- **AND** SHALL update derived projection and indexes through one application service

#### Scenario: User explicitly saves a project memory
- **WHEN** the user chooses “Remember for this project” in a session with a valid workspace
- **THEN** the system SHALL create a new active workspace memory
- **AND** SHALL reject the operation if no workspace identity can be resolved

#### Scenario: Model calls the memory tool
- **WHEN** OnePiece calls the existing memory-saving tool without an explicit UI-backed user save operation
- **THEN** the system SHALL create a reviewable candidate according to the effective policy
- **AND** the candidate SHALL NOT enter runtime prompts, `MEMORY.md`, or the retrieval index before approval

#### Scenario: Explicit save is disabled
- **WHEN** the effective explicit-save policy is disabled or the session is temporary
- **THEN** the system SHALL reject the save without writing an active record or candidate

#### Scenario: Duplicate display name
- **WHEN** a new memory uses the same display name as another memory
- **THEN** the system SHALL create a distinct immutable id unless the user explicitly selects a merge/update workflow
- **AND** SHALL NOT replace a file based on name equality

#### Scenario: Tool catalog ordering is preserved
- **WHEN** the OnePiece tool catalog is resolved
- **THEN** the memory tool SHALL retain its existing public name and position
- **AND** its implementation SHALL delegate to the personalization service rather than writing files directly

### Requirement: Candidate-producing automatic memory extraction
The system SHALL perform OnePiece compaction-triggered extraction only when allowed by the effective snapshot and SHALL return a bounded list of create, update, and archive candidate proposals. Automatic extraction SHALL NOT directly mutate active memories. It SHALL remain best effort, use one model call without tool access, and SHALL NOT fail or delay compaction materially when unavailable or unsuccessful.

#### Scenario: OnePiece extraction runs
- **WHEN** OnePiece compaction is about to replace eligible turns and effective automatic extraction is enabled
- **THEN** the system SHALL make one bounded extraction call
- **AND** SHALL persist only validated candidate proposals with source Agent, workspace, session, and source-message provenance

#### Scenario: Propose an update
- **WHEN** extraction identifies a correction to an active memory
- **THEN** it SHALL create an update candidate referencing the immutable target id and expected target revision
- **AND** SHALL leave the active memory unchanged until approval

#### Scenario: Propose removal of stale information
- **WHEN** extraction identifies an active memory that should no longer be used
- **THEN** it SHALL create an archive candidate rather than deleting the record directly

#### Scenario: Reject an invalid candidate action
- **WHEN** extraction returns an invalid id, scope, audience, type, missing field, unsafe size, or reference outside the eligible memory set
- **THEN** the system SHALL reject that proposal and preserve other valid proposals
- **AND** SHALL NOT fail compaction or the generation

#### Scenario: Extraction finds nothing
- **WHEN** extraction returns no valid candidate
- **THEN** the system SHALL create nothing and SHALL NOT treat the outcome as an error

#### Scenario: Extraction fails
- **WHEN** the extraction provider errors, times out, or returns unusable output
- **THEN** the system SHALL emit safe diagnostics and continue compaction and generation unchanged

#### Scenario: Extraction is prohibited
- **WHEN** effective automatic extraction is disabled, required capabilities are unavailable, migration is unsafe, or the session is temporary
- **THEN** the system SHALL NOT make the extraction call

### Requirement: Snapshot-scoped memory injection into the system prompt
The system SHALL inject into OnePiece only active memories eligible under the captured personalization snapshot. The always-present memory surface SHALL be a bounded index of eligible summaries; full bodies SHALL appear only through relevance selection. Memory content SHALL NOT be written into the turns list manipulated by context compaction. Index bounds SHALL include both line and byte caps and SHALL disclose truncation.

#### Scenario: Inject eligible memory alongside Skills
- **WHEN** a OnePiece snapshot permits memory read and contains eligible active memories and bound Skills
- **THEN** the system prompt SHALL include distinct Skill and memory sections
- **AND** no candidate, archived, out-of-scope, or audience-excluded memory SHALL appear

#### Scenario: Inject an index before bodies
- **WHEN** eligible memories exist
- **THEN** the system SHALL include bounded index entries containing stable id reference, name, type, description, scope hint, and age metadata
- **AND** SHALL include bodies only for memories selected for the current turn

#### Scenario: Index is truncated
- **WHEN** eligible index entries exceed the line or byte cap
- **THEN** the system SHALL include the highest-priority entries within both bounds
- **AND** SHALL state that eligible entries were omitted due to the bound

#### Scenario: Corrected memory ordering
- **WHEN** an active memory is updated through a revisioned operation
- **THEN** later indexes SHALL use the updated timestamp for ordering
- **AND** SHALL retain the immutable memory id

#### Scenario: Memory read is disabled
- **WHEN** effective memory read is disabled or the session is temporary
- **THEN** the OnePiece request SHALL contain no VaneHub long-term memory index or body

### Requirement: Paged governed memory management and maintenance
The system SHALL provide paged search and filtering, memory detail, revisioned edit, candidate review, archive/reactivate, individual delete, scoped reset preview/execute, and reconciliation for governed memories. List responses SHALL contain bounded summaries rather than every full body. Destructive maintenance SHALL use complete internal enumeration and SHALL not be limited by the UI page size or a 200-file scan cap.

#### Scenario: List a page of memories
- **WHEN** a user queries memories with search, scope, status, type, source Agent, audience, ordering, and cursor criteria
- **THEN** the system SHALL return a stable bounded page of summaries and next cursor
- **AND** SHALL not read or return each full body solely to render the list

#### Scenario: Open memory detail
- **WHEN** the user opens a memory by immutable id
- **THEN** the system SHALL return its full authorized content, scope, audience, lifecycle, provenance, revision, and timestamps

#### Scenario: Edit with the current revision
- **WHEN** the user submits a valid edit with the current expected revision
- **THEN** the system SHALL atomically update the authoritative file and derived state
- **AND** SHALL return the next revision

#### Scenario: Reject a stale memory edit
- **WHEN** an edit or candidate approval references a stale revision
- **THEN** the system SHALL return a typed conflict and SHALL NOT overwrite the current record

#### Scenario: Review a candidate
- **WHEN** the user approves, edits-and-approves, rejects, or merges a candidate
- **THEN** the system SHALL perform the selected revisioned workflow
- **AND** only approved content MAY become active

#### Scenario: Preview a scoped reset
- **WHEN** the user requests a reset preview for a scope and status filter
- **THEN** the system SHALL return exact current counts and a short-lived confirmation token
- **AND** SHALL not delete anything

#### Scenario: Execute a scoped reset
- **WHEN** the user confirms with the valid preview token and required phrase
- **THEN** the system SHALL enumerate every application-owned entry relevant to the request without a 200-file cap
- **AND** SHALL return matched, deleted-file, projection, retrieval-index, quarantine, and failure counts

#### Scenario: Reset all includes malformed owned entries
- **WHEN** the user confirms an all-memory reset
- **THEN** the maintenance path SHALL account for malformed application-owned memory files that normal parsing would skip
- **AND** SHALL permanently remove those owned files and any quarantine entries covered by the all-memory reset instead of leaving them for later rediscovery

#### Scenario: Scoped reset encounters an unclassifiable malformed file
- **WHEN** a scope-limited reset encounters a malformed owned file whose scope cannot be established safely
- **THEN** the system SHALL leave the file unavailable, report it as an explicit maintenance failure, and require repair or an all-memory reset
- **AND** SHALL NOT guess a scope or silently count the reset as complete

#### Scenario: Partial maintenance failure
- **WHEN** one file, projection row, or retrieval-index entry cannot be changed
- **THEN** the outcome SHALL report the failure and set repair-required state where consistency is uncertain
- **AND** repeated reset or reconciliation SHALL be idempotent

### Requirement: Web runtime governed memory parity
The Web/mock runtime SHALL implement the same scoped memory policy, session modes, candidate workflow, revision conflicts, paging, reset preview/execute, and reconciliation result contracts as the desktop runtime without reading native files, SQLite, a real retrieval index, a real provider, or a real CLI process.

#### Scenario: Web mock resolves memory policy
- **WHEN** a mock generation starts for an Agent, workspace, and session mode
- **THEN** the Web adapter SHALL deterministically resolve equivalent memory-read, explicit-save, automatic-extraction, global-memory-access, scope, and audience behavior

#### Scenario: Web mock manages candidates
- **WHEN** a mock candidate is listed, approved, rejected, or conflicts with a newer target revision
- **THEN** the Web adapter SHALL expose the same observable result shape as the desktop service

#### Scenario: Web mock resets a scope
- **WHEN** a reset preview and valid execution are requested in Web/mock mode
- **THEN** the adapter SHALL return deterministic exact counts and a structured outcome
- **AND** SHALL NOT claim that native files or retrieval entries were changed

### Requirement: Scoped memory policy controls
The system SHALL replace the former single host-level memory toggle as the runtime source of truth with scoped policy controls for memory read, explicit save, automatic extraction, and global-memory access. The UI MAY present a global master control for convenience, but it SHALL edit the global policy and SHALL not erase narrower overrides.

#### Scenario: Disable global memory read
- **WHEN** the user disables memory read at global scope
- **THEN** Agents that inherit the global value SHALL receive no VaneHub long-term memory
- **AND** an explicit higher-precedence enabled override MAY re-enable read except where session mode imposes a hard restriction

#### Scenario: Disable explicit save only
- **WHEN** explicit save is disabled but memory read remains enabled
- **THEN** eligible existing memories MAY still be injected
- **AND** new explicit active memories SHALL be rejected

#### Scenario: Disable automatic extraction only
- **WHEN** automatic extraction is disabled but read and explicit save remain enabled
- **THEN** the system SHALL skip OnePiece and CLI automatic extraction
- **AND** SHALL preserve permitted manual creation and recall

#### Scenario: Disable global-memory access
- **WHEN** global-memory access resolves disabled for a workspace or Agent
- **THEN** global memories SHALL be excluded while matching workspace memories MAY remain eligible

#### Scenario: Re-enable a policy
- **WHEN** a disabled policy dimension is re-enabled
- **THEN** eligible stored active memories SHALL become available again without recreating them

### Requirement: Tool-assisted extraction policy dimension
The system SHALL represent tool-assisted automatic extraction as an explicit OnePiece-capable policy dimension or capability-aware subsetting of automatic extraction, with inheritance and clear UI labeling. It SHALL not be described as controlling CLI extraction when it does not.

#### Scenario: Disable OnePiece tool-assisted extraction
- **WHEN** OnePiece compaction includes tool calls and the effective tool-assisted extraction policy is disabled
- **THEN** the system SHALL skip extraction for those compacted turns

#### Scenario: Non-tool OnePiece extraction remains allowed
- **WHEN** compacted turns contain no tool call and ordinary OnePiece automatic extraction is enabled
- **THEN** the tool-assisted sub-policy SHALL not suppress extraction

#### Scenario: CLI extraction is governed separately
- **WHEN** a CLI turn completes
- **THEN** OnePiece's tool-assisted extraction sub-policy SHALL NOT control that CLI turn
- **AND** the selected CLI Agent's automatic-extraction policy and capability SHALL govern it

#### Scenario: UI explains Agent applicability
- **WHEN** the policy view renders this control for an Agent that does not use OnePiece compaction
- **THEN** the UI SHALL hide or disable it with a specific applicability explanation

### Requirement: Snapshot-scoped memory injection into CLI prompts
The system SHALL prepend a bounded index of active memories eligible under the captured personalization snapshot to every message delivered through a compatible VaneHub-managed CLI adapter. The index SHALL follow resolved custom instructions and precede Prompt-Hook-assembled content. VaneHub SHALL not inject full memory bodies unless the runtime capability explicitly supports them and a later specification defines the behavior.

#### Scenario: Inject a scoped CLI index
- **WHEN** a CLI message snapshot permits memory read and eligible memories exist
- **THEN** the final CLI text SHALL contain the bounded eligible-memory index after custom instructions and before Prompt Hook output
- **AND** SHALL exclude candidates, archived records, wrong-workspace records, and audience-excluded records

#### Scenario: Inject on every CLI turn
- **WHEN** a CLI session sends multiple messages
- **THEN** each turn SHALL resolve and inject its own snapshot rather than relying on first-turn state

#### Scenario: Project-only CLI session
- **WHEN** a CLI session uses `project-only`
- **THEN** its index SHALL contain only matching workspace memories

#### Scenario: Temporary or disabled CLI memory
- **WHEN** the CLI snapshot disables memory read or uses temporary mode
- **THEN** the final CLI text SHALL omit VaneHub memory content

#### Scenario: Preserve original Prompt Hook input
- **WHEN** the memory index is prepended
- **THEN** Prompt Hook template variables for the user message SHALL still receive the original user input

#### Scenario: Do not modify CLI-owned memory
- **WHEN** VaneHub injects its memory index
- **THEN** it SHALL NOT create, edit, delete, or claim ownership of the CLI's native memory or instruction files

### Requirement: Candidate extraction for VaneHub-managed CLI Agents
The system SHALL attempt best-effort candidate extraction after a successful turn from every compatible VaneHub-managed CLI Agent when its effective automatic-extraction policy is enabled and a valid extraction provider is available. Extraction SHALL use the actual CLI Agent, workspace, session, and source message provenance and SHALL never change the already completed CLI response.

#### Scenario: Extract candidates after a successful CLI turn
- **WHEN** a compatible CLI turn completes successfully and effective extraction is enabled
- **THEN** the system SHALL make the bounded extraction call through the configured extraction provider
- **AND** SHALL persist validated create/update/archive proposals as candidates attributed to the actual CLI Agent

#### Scenario: CLI candidate uses workspace scope by default
- **WHEN** a CLI extraction produces a project-related candidate in a session with a workspace
- **THEN** the candidate SHALL default to that workspace scope for user review
- **AND** SHALL not become globally active without explicit approval of global scope

#### Scenario: CLI extraction provider is unavailable
- **WHEN** no valid OnePiece extraction provider credential/configuration is available
- **THEN** the system SHALL skip extraction, preserve the CLI response, and expose a safe diagnostic status

#### Scenario: CLI extraction is disabled or temporary
- **WHEN** effective extraction is disabled or the session uses temporary mode
- **THEN** the system SHALL not make the extraction call or create candidates

#### Scenario: Dynamically registered CLI Agent
- **WHEN** a new CLI Agent declares extraction support and uses the shared adapter
- **THEN** it SHALL participate without a new hard-coded extraction branch

### Requirement: Coordinated derived-state revocation and reconciliation
The system SHALL coordinate individual delete, scoped reset, archive, migration, and reconciliation with the retrieval index so that an ineligible or deleted memory cannot be recalled from an orphaned derived entry. The authoritative Markdown record, SQLite projection, `MEMORY.md`, and retrieval index SHALL be reconciled through one application service.

#### Scenario: Delete an active memory
- **WHEN** an active memory is deleted with a valid revision
- **THEN** the system SHALL remove its authoritative file and projection entry
- **AND** SHALL remove its derived index line and revoke its retrieval entry

#### Scenario: Archive an active memory
- **WHEN** a memory is archived
- **THEN** the system SHALL remove it from `MEMORY.md` and active retrieval eligibility
- **AND** SHALL retain the record for management and possible reactivation

#### Scenario: Reset many memories
- **WHEN** a scoped or all-memory reset deletes multiple records
- **THEN** the application service SHALL bulk revoke or idempotently revoke every affected retrieval entry
- **AND** SHALL report the count and failures

#### Scenario: Derived revocation fails
- **WHEN** the authoritative delete or archive succeeds but retrieval-index revocation fails
- **THEN** the memory SHALL remain excluded by authoritative eligibility checks
- **AND** the system SHALL set repair-required state and retry through reconciliation

#### Scenario: Reconciliation finds an orphan
- **WHEN** reconciliation finds a retrieval entry with no eligible authoritative memory
- **THEN** it SHALL revoke the orphan without restoring the memory

### Requirement: Memories are immutable-id addressable files
The system SHALL store each governed memory as one Markdown file named from an immutable UUID/ULID memory id. The file SHALL contain validated v2 frontmatter and body content. The immutable id, not the display name or directory-relative user-derived path, SHALL address read, update, review, archive, and delete operations. Markdown content SHALL remain authoritative, with SQLite and retrieval structures treated as derived projections.

#### Scenario: Create a memory file
- **WHEN** an active memory or candidate is persisted through the application service
- **THEN** the system SHALL allocate an immutable id and use an id-derived filename
- **AND** SHALL use create-new semantics so an existing file is never replaced accidentally

#### Scenario: Rename a memory
- **WHEN** the user edits only a memory's display name
- **THEN** the immutable id and filename SHALL remain unchanged
- **AND** the revision SHALL advance

#### Scenario: Update by immutable id
- **WHEN** a valid update references an immutable id and current revision
- **THEN** the system SHALL atomically replace only that record's file and derived projection

#### Scenario: Malformed v2 file
- **WHEN** enumeration encounters absent, invalid, inconsistent, or unsafe v2 frontmatter
- **THEN** the file SHALL NOT become active or injectable
- **AND** maintenance SHALL expose it as malformed or quarantined without stopping other valid records

#### Scenario: Duplicate display names
- **WHEN** two valid records share a display name
- **THEN** both SHALL remain independently addressable and manageable by immutable id

### Requirement: Governed memory type taxonomy
The system SHALL preserve the four memory types `user`, `feedback`, `project`, and `reference`. New v2 active memories and approved candidates SHALL require one recognized type. Legacy records with an absent or unknown type MAY migrate as explicitly `untyped` compatibility records but SHALL be visible for correction and SHALL not cause enumeration failure.

#### Scenario: Save a recognized type
- **WHEN** a new active memory or candidate declares a recognized type
- **THEN** the system SHALL preserve it through persistence, filtering, preview, and injection

#### Scenario: Reject an unknown new type
- **WHEN** a new v2 create or approval declares an unsupported type
- **THEN** the system SHALL reject it with a typed validation error

#### Scenario: Migrate an unknown legacy type
- **WHEN** a legacy memory has no recognized type
- **THEN** migration SHALL mark it `untyped` rather than guessing a type or discarding the content
- **AND** the management UI SHALL allow the user to assign a supported type

### Requirement: Derived active-memory index file
The system SHALL maintain `MEMORY.md` as a derived bounded index of active governed memories only. It SHALL contain one pointer/hook line per included active memory, SHALL contain no memory body or frontmatter, and SHALL be rebuilt from authoritative records and scope-aware metadata. Candidates, archived records, malformed files, and quarantined files SHALL NOT appear.

#### Scenario: Activate a memory
- **WHEN** a memory becomes active
- **THEN** reconciliation or the coordinated write path SHALL add exactly one id-addressed index line

#### Scenario: Archive or delete a memory
- **WHEN** an active memory is archived or deleted
- **THEN** its index line SHALL be removed

#### Scenario: Index and authoritative records disagree
- **WHEN** `MEMORY.md` is missing, stale, duplicated, or references an ineligible record
- **THEN** reconciliation SHALL regenerate it from active authoritative records
- **AND** SHALL not treat the index as authoritative

#### Scenario: Index exceeds runtime bounds
- **WHEN** the complete active index exceeds a runtime adapter's line or byte budget
- **THEN** persisted `MEMORY.md` MAY remain complete within its own safe file limit
- **AND** the runtime SHALL build a bounded eligible view with explicit truncation

### Requirement: Governed model-side memory correction
The system SHALL prevent OnePiece generic file tools from bypassing governed v2 memory revision, scope, review, and index invariants. Model-proposed corrections or removals SHALL use typed personalization operations that create update/archive candidates unless an explicit user-confirmed application action authorizes direct mutation. Approval behavior for paths outside the governed memory directory SHALL remain unchanged.

#### Scenario: Model proposes a correction
- **WHEN** the model determines that an active memory should change
- **THEN** it SHALL submit an update candidate referencing the immutable target id and revision
- **AND** SHALL not directly overwrite the Markdown file through a generic file tool

#### Scenario: Model proposes removal
- **WHEN** the model determines that an active memory is stale or wrong
- **THEN** it SHALL submit an archive candidate
- **AND** SHALL not delete the record before review

#### Scenario: Generic read is allowed by policy
- **WHEN** OnePiece needs the body of an eligible memory and the runtime adapter authorizes the read
- **THEN** the system MAY return the body through the personalization API
- **AND** SHALL not grant arbitrary directory write authority

#### Scenario: Write outside memory storage is unaffected
- **WHEN** the model writes to a path outside governed memory storage
- **THEN** existing tool permission and approval behavior SHALL remain unchanged

#### Scenario: Memory mutation is prohibited
- **WHEN** effective memory write is disabled or the session is temporary
- **THEN** model-originated memory correction or archive proposals SHALL be rejected without persistent mutation

### Requirement: Migration to governed v2 memory storage
The system SHALL retain the existing idempotent row-store-to-file migration behavior and SHALL additionally migrate every legacy path-addressed memory file to the v2 immutable-id governed format without a model call. Migration SHALL preserve content and provenance, default valid legacy records to active global scope and all-Agent audience for compatibility, quarantine malformed records, rebuild derived state, and remain idempotent after interruption.

#### Scenario: Existing row first becomes a governed file
- **WHEN** a pre-file memory row remains on upgrade
- **THEN** the system SHALL preserve its content and provenance through the existing conversion and then produce one v2 immutable-id file
- **AND** SHALL not create duplicate final records

#### Scenario: Valid legacy file migrates
- **WHEN** migration encounters a valid path-addressed legacy memory file
- **THEN** it SHALL create a verified v2 file with a new immutable id, active global scope, all-Agent audience, preserved metadata/content, and legacy-migration source
- **AND** SHALL remove the legacy source only after a migration manifest and successful verification exist

#### Scenario: Malformed legacy file migrates safely
- **WHEN** a legacy file cannot be parsed or validated
- **THEN** the system SHALL move or copy it to quarantine with a diagnostic record
- **AND** SHALL not activate, inject, or silently delete it

#### Scenario: Migration is interrupted
- **WHEN** the process stops after some records have converted
- **THEN** the next run SHALL recognize completed v2 records and migration manifest entries
- **AND** SHALL not duplicate or overwrite user-edited v2 records

#### Scenario: Migration exceeds 200 files
- **WHEN** the legacy directory contains more than 200 files
- **THEN** migration SHALL enumerate every application-owned entry rather than stopping at the old query cap

#### Scenario: One record fails
- **WHEN** one record cannot be migrated
- **THEN** the system SHALL continue other records, mark migration or repair state accurately, and keep unsafe memory unavailable
- **AND** SHALL not abort unrelated application startup

### Requirement: Eligibility-filtered relevance-selected memory bodies
The system SHALL select a bounded number of OnePiece memory bodies only from active records already eligible under the captured snapshot. Selection SHALL operate on immutable id, name, type, description, scope hint, and age without exposing bodies to the selection manifest. It SHALL return no body when none is clearly useful. Failure SHALL degrade to eligible index-only injection.

#### Scenario: Select relevant eligible memories
- **WHEN** OnePiece selection judges eligible active memories useful
- **THEN** the system SHALL load and inject no more than the configured bound by immutable id
- **AND** SHALL retain the eligible index

#### Scenario: Ineligible memory cannot be selected
- **WHEN** a memory is global in project-only mode, belongs to another workspace, excludes the Agent, is a candidate, or is archived
- **THEN** it SHALL not appear in the selection manifest or selected bodies

#### Scenario: Nothing is clearly relevant
- **WHEN** selection finds no clearly useful eligible memory
- **THEN** the system SHALL inject no bodies and preserve the eligible index without error

#### Scenario: Selection fails or names an invalid id
- **WHEN** selection errors, times out, returns unusable data, or references an id absent from the eligible set
- **THEN** the system SHALL discard invalid selections, use eligible index-only behavior, and continue generation

#### Scenario: Memory read is disabled
- **WHEN** the snapshot disables memory read or uses temporary mode
- **THEN** the system SHALL not perform relevance selection

### Requirement: Surfaced memory id and revision exclusion
The system SHALL track immutable memory id and revision for bodies surfaced within a session and SHALL exclude an unchanged id/revision pair from later OnePiece selection before applying the selection bound.

#### Scenario: Unchanged memory is not re-selected
- **WHEN** the same memory id and revision was injected earlier in the session
- **THEN** it SHALL not be offered to later selection
- **AND** the selection bound SHALL remain available for unseen eligible records

#### Scenario: New session starts fresh
- **WHEN** a new session begins
- **THEN** eligible memories SHALL have no surfaced marker for that session

#### Scenario: Updated revision becomes eligible
- **WHEN** an already surfaced memory is updated to a new revision and remains eligible
- **THEN** the new id/revision pair MAY be selected again

#### Scenario: Scope changes make a surfaced memory ineligible
- **WHEN** policy, session mode, scope, audience, archive, or delete state later excludes a surfaced memory
- **THEN** it SHALL not be offered regardless of surfaced tracking

### Requirement: Injected memory age, staleness, and data labeling
The system SHALL annotate every injected memory body with human-readable age derived from its authoritative updated or verified timestamp. A body older than the configured staleness threshold SHALL include a verify-before-asserting caveat. Scope and provenance labels SHALL be data labels and SHALL not elevate memory content into higher-priority instructions.

#### Scenario: Inject a stale memory
- **WHEN** an eligible memory older than the staleness threshold is injected
- **THEN** its wrapper SHALL state human-readable age and that point-in-time claims must be verified against current state

#### Scenario: Inject a fresh memory
- **WHEN** an eligible memory is within the threshold
- **THEN** its wrapper SHALL state human-readable age without the stale caveat

#### Scenario: Candidate is never caveated or injected
- **WHEN** a record remains a candidate
- **THEN** it SHALL not reach the injection stage regardless of age

#### Scenario: Memory content contains imperative text
- **WHEN** an injected memory body contains instruction-like language
- **THEN** the wrapper SHALL identify it as recalled user/project data rather than product, safety, role, or tool authorization

### Requirement: Web runtime parity for governed memory selection
The Web/mock runtime SHALL expose equivalent policy eligibility, bounded index, selected-body event shape, surfaced id/revision tracking, staleness metadata, and disabled/project-only/temporary behavior without a real provider call.

#### Scenario: Mock selection uses eligible records
- **WHEN** a mock OnePiece generation has active eligible memories
- **THEN** the Web adapter SHALL deterministically simulate bounded index and selected-body behavior through the same observable contracts
- **AND** SHALL exclude records using the same scope, audience, status, and session-mode rules

#### Scenario: Mock memory read is suppressed
- **WHEN** effective memory read is disabled or the session is temporary
- **THEN** the Web adapter SHALL emit no simulated memory selection

#### Scenario: Mock project-only behavior
- **WHEN** a mock session uses project-only mode
- **THEN** only matching workspace records SHALL participate

## MODIFIED Requirements

### Requirement: Memory recall participates through an independent context budget
Eligible active cross-session memory recall SHALL expose bounded Context Engine candidates and SHALL be ranked and budgeted independently from code evidence after policy filtering. Candidate, archived, malformed, quarantined, out-of-scope, and audience-excluded records SHALL consume no recall budget. Selection diagnostics and persisted manifest metadata SHALL not contain full memory bodies.

#### Scenario: Relevant memory competes with code evidence
- **WHEN** eligible relevant memory and code candidates are available
- **THEN** memory SHALL consume only its versioned source allocation unless protected by an authoritative rule
- **AND** its body SHALL NOT appear in selection diagnostics or persisted manifest metadata

#### Scenario: Ineligible records do not consume budget
- **WHEN** records are excluded by status, scope, audience, session mode, or policy
- **THEN** they SHALL be removed before context ranking and budget accounting

#### Scenario: Memory budget is exhausted
- **WHEN** eligible memory candidates exceed the memory allocation
- **THEN** the context engine SHALL apply its documented ranking/bounding behavior without borrowing unbounded capacity from code evidence
