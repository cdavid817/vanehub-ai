# unified-personalization-governance Specification

## Purpose
Govern how personalization data — cross-session memories, custom instructions, and their policies — is scoped, audited, and reviewed across every VaneHub-managed Agent. Each memory carries a scope (global or one workspace) and an audience (all Agents or a named subset) that the trusted runtime enforces at injection time, ordered lifecycle → read policy → scope → audience; automatic paths only propose candidates that a person approves, session modes (standard, project-only, temporary) can only narrow the resolved policy, and files remain the authoritative surface with every index, projection, and retrieval entry derived from them.
## Requirements
### Requirement: Unified personalization coverage for VaneHub-managed Agents
The system SHALL resolve personalization through one native governance boundary for every Agent generation started through a VaneHub-managed runtime adapter, using stable registry Agent identity rather than a hard-coded built-in Agent list.

#### Scenario: Resolve personalization for OnePiece
- **WHEN** a OnePiece generation starts through the standard VaneHub runtime path
- **THEN** the system SHALL resolve a personalization snapshot for `agentId = onepiece`
- **AND** the OnePiece adapter SHALL consume only that snapshot for VaneHub-owned custom instructions and long-term memory access

#### Scenario: Resolve personalization for a built-in CLI Agent
- **WHEN** Claude Code, Codex, OpenCode, Gemini CLI, or Antigravity starts a message through its VaneHub runtime adapter
- **THEN** the system SHALL resolve a personalization snapshot for that CLI's stable Agent id
- **AND** SHALL apply the runtime capability contract rather than branching on the Agent display name

#### Scenario: Resolve personalization for a newly registered Agent
- **WHEN** a new API or CLI Agent is registered with personalization capability metadata and uses the standard generation service
- **THEN** the system SHALL make the Agent available for policy resolution and UI selection without requiring a new hard-coded personalization list

#### Scenario: Preserve external CLI independence
- **WHEN** a user launches a CLI directly outside VaneHub
- **THEN** VaneHub SHALL NOT claim that its personalization policy is applied to that process
- **AND** SHALL NOT modify the CLI's native instruction or memory files as part of this capability

### Requirement: Deterministic personalization scopes and precedence
The system SHALL support global, Agent, workspace, and workspace-Agent policy scopes and SHALL resolve them in deterministic precedence before applying any session override or hard session-mode restriction.

#### Scenario: Resolve ordinary precedence
- **WHEN** global, Agent, workspace, and workspace-Agent records all exist for a generation context
- **THEN** the system SHALL apply precedence in this order: built-in safe defaults, global, Agent, workspace, workspace-Agent, session override, session-mode restriction
- **AND** the effective preview SHALL identify the source of every resolved value

#### Scenario: Inherit an unset override
- **WHEN** a non-global scope stores `inherit` for a policy field
- **THEN** that scope SHALL leave the lower-precedence resolved value unchanged

#### Scenario: Use stable workspace identity
- **WHEN** a local or remote workspace is resolved for personalization
- **THEN** the system SHALL use a stable local workspace key rather than comparing only the display path
- **AND** equal remote paths on different connection identities SHALL NOT share a workspace scope

#### Scenario: Resolve a multi-Agent seat
- **WHEN** a multi-Agent seat starts a turn
- **THEN** the system SHALL resolve the shared session and workspace context with that seat's own stable Agent id
- **AND** SHALL NOT reuse another seat's Agent-specific policy result

### Requirement: Instruction merge semantics
The system SHALL resolve scoped custom instructions through explicit `inherit`, `append`, `replace`, and `disabled` merge modes while keeping product-owned core instructions outside user-personalization control.

#### Scenario: Append scoped instructions
- **WHEN** a higher-precedence scope uses `append` with non-empty instruction fields
- **THEN** the system SHALL retain inherited user instruction segments and append the higher-precedence segments in resolution order

#### Scenario: Replace inherited user instructions
- **WHEN** a higher-precedence scope uses `replace`
- **THEN** the system SHALL discard lower-precedence user instruction segments and use the replacement fields
- **AND** SHALL NOT remove core product, safety, role, or runtime instructions

#### Scenario: Disable user instructions for a request
- **WHEN** the effective instruction mode is `disabled`
- **THEN** the system SHALL omit all user-personalization instruction segments for the request
- **AND** SHALL preserve non-personalization instructions

#### Scenario: Explain instruction provenance
- **WHEN** the user previews an effective policy
- **THEN** the system SHALL return each included or excluded user instruction segment with its scope and reason
- **AND** SHALL NOT return hidden core system-prompt content

### Requirement: Immutable per-generation personalization snapshot
The system SHALL capture one immutable effective-personalization snapshot at the start of each generation or Agent seat turn and SHALL use that snapshot for all VaneHub-owned personalization decisions within that operation.

#### Scenario: Policy changes during generation
- **WHEN** a policy is saved after a generation has captured its snapshot
- **THEN** the active generation SHALL continue with the captured revision
- **AND** the new policy SHALL apply only to later generations

#### Scenario: Memory state changes during generation
- **WHEN** an eligible memory is edited, archived, or deleted after snapshot capture
- **THEN** the active generation SHALL NOT silently rebuild its prompt from a different policy revision
- **AND** later generations SHALL use the updated memory state

#### Scenario: Capture a diagnostic revision token
- **WHEN** a snapshot is resolved
- **THEN** it SHALL contain a stable safe revision token derived from the contributing policy revisions and session context
- **AND** the token SHALL NOT include instruction content, memory content, credentials, or raw filesystem paths

### Requirement: Session personalization modes
The system SHALL support `standard`, `project-only`, and `temporary` personalization modes as durable session behavior.

#### Scenario: Use standard mode
- **WHEN** a session uses `standard`
- **THEN** the system SHALL apply resolved custom instructions
- **AND** MAY read global and workspace active memories and create memories according to the effective policy

#### Scenario: Use project-only mode
- **WHEN** a session with a valid workspace uses `project-only`
- **THEN** the system SHALL apply resolved custom instructions
- **AND** SHALL exclude global memories from read, explicit save, and automatic extraction behavior
- **AND** SHALL constrain long-term memory writes to the active workspace

#### Scenario: Reject project-only mode without a workspace
- **WHEN** session creation or update requests `project-only` without a resolvable workspace
- **THEN** the system SHALL reject the request with a typed validation error

#### Scenario: Use temporary mode
- **WHEN** a session uses `temporary`
- **THEN** the system SHALL continue to apply resolved custom instructions
- **AND** SHALL perform no long-term memory read, active save, candidate creation, automatic extraction, or retrieval-index write
- **AND** SHALL leave current-session history and runtime-owned internal compaction unchanged

### Requirement: Capability-aware Agent controls
The system SHALL expose personalization capability metadata for registered Agents and SHALL use it to determine runtime behavior and available UI controls.

#### Scenario: Agent supports index-only memory injection
- **WHEN** an Agent declares memory-index support but not selected-memory-body support
- **THEN** the runtime SHALL inject only the eligible bounded index
- **AND** the UI SHALL NOT imply that selected bodies are injected

#### Scenario: Agent does not support automatic extraction
- **WHEN** an Agent declares no automatic-extraction capability
- **THEN** the policy UI SHALL show extraction as unavailable with a reason
- **AND** the runtime SHALL NOT attempt extraction even if an inherited policy value is enabled

#### Scenario: Agent capability changes after discovery
- **WHEN** Agent discovery returns updated capability metadata
- **THEN** subsequent policy previews and generations SHALL use the updated capabilities
- **AND** persisted policy overrides SHALL remain stored for possible future support rather than being silently deleted

### Requirement: Dedicated revisioned personalization persistence
The system SHALL persist personalization policy through a dedicated native service and SHALL use optimistic concurrency instead of whole-application-settings replacement.

#### Scenario: Save a policy scope
- **WHEN** a user saves a valid policy patch with the current expected revision
- **THEN** the system SHALL atomically persist only that typed policy scope
- **AND** SHALL return the new revision and effective metadata

#### Scenario: Reject a stale policy edit
- **WHEN** a policy patch carries an expected revision older than the persisted revision
- **THEN** the system SHALL return a typed conflict with the safe current policy record
- **AND** SHALL NOT apply last-response-wins replacement

#### Scenario: Save independent scopes concurrently
- **WHEN** two valid requests update different policy scope keys concurrently
- **THEN** each request SHALL be evaluated against its own scope revision
- **AND** one response SHALL NOT replace the other scope's state

#### Scenario: Preserve Web/mock parity
- **WHEN** policy operations execute through the Web/mock adapter
- **THEN** the adapter SHALL preserve the same scope, revision, validation, conflict, and effective-preview contract without claiming SQLite persistence

### Requirement: Fail-closed personalization fallback
The system SHALL fail closed for VaneHub-owned long-term memory and user instructions when no validated personalization policy can be loaded, while allowing the underlying Agent generation to continue.

#### Scenario: Use a validated last-known-good policy
- **WHEN** a transient policy read fails after a validated policy has been cached
- **THEN** the system MAY use the last-known-good policy snapshot
- **AND** SHALL expose a safe warning that current persistence could not be read

#### Scenario: No validated policy is available
- **WHEN** policy loading fails and no validated prior policy exists
- **THEN** the system SHALL omit user-personalization instructions
- **AND** SHALL deny long-term memory read, save, candidate creation, and automatic extraction
- **AND** SHALL continue the generation without enabling memory implicitly

#### Scenario: Personalization migration is incomplete
- **WHEN** startup detects an incomplete or unsafe personalization migration
- **THEN** the system SHALL keep long-term memory unavailable until migration or reconciliation establishes a valid generation
- **AND** SHALL surface a maintenance state without preventing unrelated application use

### Requirement: Effective personalization preview
The system SHALL provide a safe preview of the effective personalization result for a selected Agent, workspace, session mode, and optional session.

#### Scenario: Preview resolution sources
- **WHEN** a user requests an effective preview
- **THEN** the system SHALL return final policy values, contributing scopes, Agent capabilities, instruction provenance, eligible and excluded memory counts, exclusion reasons, warnings, and estimated context size

#### Scenario: Preview runtime-specific behavior
- **WHEN** the selected runtime is OnePiece or a CLI adapter
- **THEN** the preview SHALL state whether VaneHub will provide selected memory bodies, an index only, or no memory
- **AND** SHALL state that CLI-internal compaction is outside VaneHub governance

#### Scenario: Redact unsafe preview data
- **WHEN** preview data is returned to the frontend
- **THEN** it SHALL exclude credentials, hidden core system instructions, unredacted trace payloads, and memory bodies not explicitly requested through an authorized detail operation

### Requirement: Personalization service boundary parity
The system SHALL expose personalization to React only through `AgentService`, with Tauri and Web/mock adapters implementing equivalent typed operations.

#### Scenario: Desktop UI loads personalization
- **WHEN** React loads or mutates personalization in the desktop runtime
- **THEN** it SHALL call `AgentService`
- **AND** only the Tauri adapter SHALL invoke native personalization commands

#### Scenario: Web/mock UI loads personalization
- **WHEN** React loads or mutates personalization in Web/mock mode
- **THEN** it SHALL use the same `AgentService` contract
- **AND** the Web/mock adapter SHALL provide deterministic equivalent paging, conflict, candidate, preview, reset, and session-mode behavior

#### Scenario: Prevent a direct component invocation
- **WHEN** a personalization React component requires native behavior
- **THEN** it SHALL NOT invoke Tauri directly or read native files

### Requirement: Personalization observability uses safe metadata
The system SHALL record safe operational metadata for policy resolution, extraction, migration, reset, and reconciliation without logging instruction bodies, memory bodies, credentials, or hidden prompts by default.

#### Scenario: Record a resolution event
- **WHEN** a snapshot is resolved
- **THEN** observability MAY record Agent id, runtime kind, session mode, policy revision token, included/excluded counts, duration, and warning codes
- **AND** SHALL NOT record user instruction or memory content under metadata-only capture

#### Scenario: Record a maintenance failure
- **WHEN** migration, reset, projection update, or retrieval-index reconciliation partially fails
- **THEN** the system SHALL record typed phase and count metadata sufficient for diagnosis
- **AND** SHALL keep sensitive paths and content behind explicit local advanced diagnostics

