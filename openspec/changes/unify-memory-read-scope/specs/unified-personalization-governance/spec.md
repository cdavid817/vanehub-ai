## MODIFIED Requirements

### Requirement: Immutable per-generation personalization snapshot
The system SHALL capture one immutable effective-personalization snapshot at the start of each generation or Agent seat turn and SHALL use that snapshot for all VaneHub-owned personalization decisions within that operation. The snapshot SHALL supply one trusted memory-read context before any memory source collection, including Context Engine. Its policy and subject are frozen; its bounded injection refs MUST NOT be treated as the complete recall universe. Each new read batch MAY pin current records within the frozen rules, and delivery MUST revalidate authoritative record status, scope, audience and pinned version. Current unsafe maintenance or invalid ownership SHALL block new delivery without silently adopting another policy.

#### Scenario: Policy changes during generation
- **WHEN** a policy is saved after a generation has captured its snapshot
- **THEN** the active generation SHALL continue with the captured revision
- **AND** the new policy SHALL apply only to later generations

#### Scenario: Memory state changes during generation
- **WHEN** an eligible memory is edited, archived, or deleted after snapshot capture
- **THEN** the active generation SHALL NOT silently rebuild its prompt from a different policy revision
- **AND** later generations SHALL use the updated memory state; pending old handles SHALL not be silently replaced, while a later read batch MAY discover a current eligible version under the same frozen policy

#### Scenario: Capture a diagnostic revision token
- **WHEN** a snapshot is resolved
- **THEN** it SHALL contain a stable safe revision token derived from the contributing policy revisions and session context
- **AND** the token SHALL NOT include instruction content, memory content, credentials, or raw filesystem paths

### Requirement: Effective personalization preview
The system SHALL provide a safe preview of the effective personalization result for a selected Agent, workspace, session mode, and optional session. The UI SHALL distinguish hypothetical previews from bound-session previews. Native bound-session preview MUST derive Agent/seat, session mode and workspace from the owning session instead of trusting caller-supplied scope fields. Hypothetical results MUST NOT establish runtime authority. Read allowance, full eligible count, bounded index count/truncation and recall availability SHALL be separate facts; an allowed empty pool MUST NOT be reported as read disabled.

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

#### Scenario: Caller supplies a mismatching session workspace
- **WHEN** a bound-session preview request attempts to describe a different workspace or mode than the stored session
- **THEN** native resolution SHALL use the actual session context or reject the mismatch rather than report it as the session effective authority

#### Scenario: Memory is allowed but none is eligible
- **WHEN** a valid policy permits read and the eligible pool is empty
- **THEN** preview SHALL report read allowed with zero eligible records and SHALL not infer a disabled policy

#### Scenario: CLI supports only an index
- **WHEN** a managed CLI has memory-index capability without a governed recall channel
- **THEN** preview SHALL state index support and recall unsupported separately

### Requirement: Personalization service boundary parity
The system SHALL expose personalization to React only through `AgentService`, with Tauri and Web/mock adapters implementing equivalent typed operations. The Web/mock implementation SHALL reproduce all reading-related policy layers, session hard restrictions, exact audience membership and bound/hypothetical preview distinctions. It SHALL use the same logical eligibility fixtures as native code without performing native IO or provider calls.

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
The system SHALL record safe operational metadata for policy resolution, extraction, migration, reset, and reconciliation without logging instruction bodies, memory bodies, credentials, or hidden prompts by default. Memory read diagnostics MAY add bounded surface kind, scope fingerprint, current health/revision, authorized candidate counts and degradation reasons. Agent-facing failures SHALL not reveal the existence, ids, names or counts of excluded records; owner-level exclusion previews SHALL retain their existing management semantics.

#### Scenario: Record a resolution event
- **WHEN** a snapshot is resolved
- **THEN** observability MAY record Agent id, runtime kind, session mode, policy revision token, included/excluded counts, duration, and warning codes
- **AND** SHALL NOT record user instruction or memory content under metadata-only capture

#### Scenario: Record a maintenance failure
- **WHEN** migration, reset, projection update, or retrieval-index reconciliation partially fails
- **THEN** the system SHALL record typed phase and count metadata sufficient for diagnosis
- **AND** SHALL keep sensitive paths and content behind explicit local advanced diagnostics
