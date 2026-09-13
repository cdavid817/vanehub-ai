## MODIFIED Requirements

### Requirement: Governed memory scope and audience
The system SHALL store every memory with an explicit `global` or `workspace` scope and an optional all-Agent or selected-Agent audience. Producing Agent, producing workspace, session, message, and save source SHALL remain provenance and SHALL NOT be substituted for the explicit scope or audience. Runtime injection SHALL filter by the captured personalization snapshot before budgeting or relevance selection. Recall, selected bodies, metadata selection and Context Engine memory collection SHALL use that same governed eligibility domain through a trusted native read context. Shared storage and provenance SHALL NOT override these conditions; worktrees SHALL retain their existing distinct workspace identities.

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

### Requirement: Snapshot-scoped memory injection into the system prompt
The system SHALL inject into OnePiece only active memories eligible under the captured personalization snapshot. The always-present memory surface SHALL be a bounded index of eligible summaries; full bodies SHALL appear only through relevance selection. Memory content SHALL NOT be written into the turns list manipulated by context compaction. Index bounds SHALL include both line and byte caps and SHALL disclose truncation. The read context SHALL be resolved before any Context Engine memory collection. Name/description/index metadata SHALL be checked against authoritative eligibility before delivery. The bounded index page MUST NOT become the complete recall allowlist; no generic compatibility view SHALL substitute for governed metadata or body reads.

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

#### Scenario: Context Engine cannot run before memory policy
- **WHEN** a OnePiece generation enables the memory Context Engine source
- **THEN** the runtime SHALL supply the same resolved read context before source collection, and a denied context SHALL prevent memory collection rather than merely hide recall from the catalog

### Requirement: Snapshot-scoped memory injection into CLI prompts
The system SHALL prepend a bounded index of active memories eligible under the captured personalization snapshot to every message delivered through a compatible VaneHub-managed CLI adapter. The index SHALL follow resolved custom instructions and precede Prompt-Hook-assembled content. VaneHub SHALL not inject full memory bodies unless the runtime capability explicitly supports them and a later specification defines the behavior. The CLI index SHALL consume the same governed metadata API and actual stable Agent/workspace identity as native injection. Index support alone MUST NOT advertise governed recall support. This change SHALL not add a CLI recall bridge or modify CLI-owned memory files.

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

### Requirement: Eligibility-filtered relevance-selected memory bodies
The system SHALL select a bounded number of OnePiece memory bodies only from active records already eligible under the captured snapshot. Selection SHALL operate on immutable id, name, type, description, scope hint, and age without exposing bodies to the selection manifest. It SHALL return no body when none is clearly useful. Failure SHALL degrade to eligible index-only injection. Selectors SHALL return immutable candidate ids or opaque id-bound handles rather than display names. Each selected handle SHALL pin revision, content hash and authority metadata and be loaded by the governed read API with the captured read context; the legacy active/global/all-Agent compatibility reader MUST NOT serve this path.

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

#### Scenario: Load an eligible workspace or selected-Agent body
- **WHEN** an eligible workspace-scoped or selected-Agent memory is selected by valid id
- **THEN** the native runtime SHALL be able to load and inject its pinned authorized body rather than drop it because it is absent from a public compatibility view

#### Scenario: Duplicate names do not misroute selection
- **WHEN** two eligible records have the same display name and different ids
- **THEN** selection SHALL identify the requested id unambiguously and SHALL not fall back to the first matching name

### Requirement: Surfaced memory id and revision exclusion
The system SHALL track immutable memory id and revision for bodies surfaced within a session and SHALL exclude an unchanged id/revision/content-hash tuple for the same subject and read context from later OnePiece selection before applying the selection bound. Surfaced state SHALL be partitioned by session, actual Agent/seat authority and workspace/mode context, with content hash included in version identity. It SHALL remain a deduplication optimization applied after current eligibility and MUST NOT cache permission or suppress another subject eligible record.

#### Scenario: Unchanged memory is not re-selected
- **WHEN** the same memory id, revision and content hash was injected earlier in the session for the same Agent/seat authority and read context
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

#### Scenario: Another Agent uses the same group session
- **WHEN** a different Agent or seat has an eligible record already surfaced to the previous subject
- **THEN** the new subject SHALL not inherit that prior surfaced suppression or authorization

### Requirement: Derived active-memory index file
The system SHALL maintain `MEMORY.md` as a derived bounded index of active governed memories only. It SHALL contain one pointer/hook line per included active memory, SHALL contain no memory body or frontmatter, and SHALL be rebuilt from authoritative records and scope-aware metadata. Candidates, archived records, malformed files, and quarantined files SHALL NOT appear. The derived host file SHALL not itself establish a runtime read domain. Every VaneHub delivery of its information SHALL rebuild an authorized view through the governed metadata API; owner-wide persisted entries MUST NOT be copied wholesale into Agent prompts.

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
