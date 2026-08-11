## Purpose

Enables native API Agents to delegate bounded tasks to effective Utility Skills through isolated, permission-aware child-Agent attempts with structured results and complete lifecycle control.

## ADDED Requirements

### Requirement: Utility delegation eligibility
The system SHALL permit delegation only to an enabled, available, trusted, effective Skill with `type: utility` that is assigned to the calling native API Agent in the active canonical workspace context. It SHALL resolve canonical ids and aliases using the effective Skill runtime and SHALL return a structured refusal for every ineligible target.

#### Scenario: Eligible Utility selected
- **WHEN** a native API Agent delegates to an enabled, trusted, effective Utility Skill assigned to that stable Agent id
- **THEN** the system SHALL resolve the current Overlay-applied Utility revision and proceed to permission evaluation

#### Scenario: Role Skill refused
- **WHEN** `delegate_skill` targets a Role Skill
- **THEN** the system SHALL reject the call without creating a child attempt or incrementing Utility use

#### Scenario: Unassigned Utility refused
- **WHEN** an API Agent targets a Utility Skill that is not assigned to its stable Agent id
- **THEN** the system SHALL reject the call without exposing the Utility instructions

#### Scenario: Untrusted or conflicted Utility refused
- **WHEN** the effective Utility is untrusted, unavailable, disabled, invalid, or has no deterministic Overlay-applied content
- **THEN** delegation SHALL fail closed with a safe unavailable reason

#### Scenario: CLI caller unsupported
- **WHEN** a third-party CLI Agent lacks a native delegation adapter
- **THEN** the system SHALL NOT advertise or inject VaneHub's `delegate_skill` tool into that CLI runtime

### Requirement: Utility delegation metadata
A Utility Skill MAY declare a delegation contract containing stable tool-capability ids and lower requested limits for model rounds, duration, context, and output. Unknown or prohibited capability ids SHALL make delegation unavailable. Missing declarations SHALL use the platform's read-only default and platform limits.

#### Scenario: Read-only default
- **WHEN** a Utility Skill omits its delegation contract
- **THEN** the child SHALL receive only the platform's default read-only tools and bounded defaults

#### Scenario: Utility requests lower limits
- **WHEN** a Utility declares valid limits lower than platform ceilings
- **THEN** the child attempt SHALL enforce the declared lower limits

#### Scenario: Utility requests excessive limits
- **WHEN** a Utility declares a limit above a platform ceiling
- **THEN** the effective limit SHALL remain at the platform ceiling and management responses SHALL identify the capped value

#### Scenario: Unknown capability
- **WHEN** a Utility declares an unknown or prohibited tool-capability id
- **THEN** the Utility SHALL be unavailable for delegation until its metadata is corrected

### Requirement: Fixed delegation tool contract
Native API Agents SHALL receive one fixed-schema `delegate_skill` tool rather than one provider tool per Utility. Its input SHALL contain a bounded Skill id or alias, task, optional context summary, and optional logical Skill resource references. Inventory changes SHALL NOT alter the provider tool schema.

#### Scenario: Utility inventory changes
- **WHEN** Utility Skills are installed, removed, shadowed, enabled, disabled, assigned, or updated
- **THEN** the `delegate_skill` provider schema SHALL remain unchanged

#### Scenario: Malformed delegation input
- **WHEN** a delegation call exceeds field limits, contains an unknown field, or provides a malformed resource reference
- **THEN** the system SHALL reject it before permission evaluation or child creation

#### Scenario: Alias accepted
- **WHEN** the tool receives an unambiguous alias for an eligible Utility
- **THEN** the result and all persisted records SHALL use the canonical Skill id

### Requirement: Delegation start approval
Starting a Utility delegation SHALL be a unified permission action that defaults to `Ask`. The evaluation resource SHALL identify the canonical Utility, effective revision, parent Agent, workspace scope, and effective capability ceiling. A remembered permission grant MAY resolve the start action, but SHALL NOT approve any child tool action.

#### Scenario: Start requires approval
- **WHEN** no policy or remembered grant resolves an eligible delegation start to Allow or Deny
- **THEN** the parent generation SHALL pause and create a pending approval before any child model request begins

#### Scenario: Start denied
- **WHEN** policy or user decision denies the delegation start
- **THEN** no child attempt SHALL run and the parent SHALL receive a structured denied result

#### Scenario: Start allowed by remembered grant
- **WHEN** an applicable remembered grant allows the same delegation resource and scope
- **THEN** the child attempt MAY start without a new start-approval prompt
- **AND** every child tool action SHALL still be evaluated independently

#### Scenario: Stop while start approval pending
- **WHEN** the parent generation is stopped while delegation approval is pending
- **THEN** the pending approval SHALL become stale, the delegation SHALL not start, and the parent call SHALL end as cancelled

### Requirement: Isolated child-Agent attempt
Each accepted delegation SHALL create a distinct child attempt linked to the parent generation, stable parent Agent id, canonical Utility id, effective Utility revision, workspace, and permission principal. The child SHALL use the parent's captured provider interface, model, and credential handle without exposing credential material to its prompt or result.

#### Scenario: Child uses captured provider snapshot
- **WHEN** delegation starts and the user later changes the session's selected model or provider profile
- **THEN** the running child SHALL continue with the provider and model snapshot captured at attempt creation

#### Scenario: Distinct attempt identity
- **WHEN** the same parent invokes the same Utility twice
- **THEN** the system SHALL create distinct attempt and execution identifiers while retaining the same canonical Utility and child-principal identity

#### Scenario: Provider unavailable before start
- **WHEN** the captured provider or model is no longer available before the first child request
- **THEN** the attempt SHALL fail without falling back to a different provider or model silently

### Requirement: Bounded child context
The child prompt SHALL contain the effective Utility instructions, the bounded delegated task, explicit context summary, permitted logical resources, workspace identity metadata, and output contract. It SHALL NOT automatically include the complete parent transcript, hidden reasoning, provider credentials, unrelated memories, or unrequested files.

#### Scenario: Minimal context by default
- **WHEN** a delegation call supplies only a task
- **THEN** the child SHALL receive the Utility instructions and task but not the parent conversation transcript

#### Scenario: Explicit context included
- **WHEN** the parent supplies a bounded context summary and valid logical resource references
- **THEN** the child SHALL receive only those values within the effective context ceiling

#### Scenario: Context budget exceeded
- **WHEN** supplied context would exceed the effective limit
- **THEN** the system SHALL reject the call rather than silently include a partial or different context

#### Scenario: Resource is not permitted
- **WHEN** a supplied logical resource is stale, outside the effective Utility package, or unavailable to the calling context
- **THEN** the system SHALL reject it without reading the target

### Requirement: Effective child tool ceiling
The child tool catalog SHALL be the intersection of platform-allowed child tools, the parent permission mode, the Utility's declared capabilities, effective trust and availability, and unified permission policy. The child SHALL NOT receive `delegate_skill`, dynamic script tools, or undeclared capabilities.

#### Scenario: Parent Plan mode ceiling
- **WHEN** a read-only Utility is delegated from a parent generation in Plan mode
- **THEN** the child SHALL receive only Plan-compatible read-only tools

#### Scenario: Utility declares file write under Standard mode
- **WHEN** an eligible Utility declares file-write capability and the parent mode permits writes
- **THEN** the child MAY receive the bounded file-write operation
- **AND** each write SHALL still pass unified permission evaluation and approval

#### Scenario: Parent mode denies declared capability
- **WHEN** a Utility declares a capability prohibited by the parent permission mode
- **THEN** that capability SHALL be absent from the child catalog and rejected again at dispatch

#### Scenario: MCP excluded by default
- **WHEN** a Utility does not have a future explicit MCP delegation capability
- **THEN** the child SHALL receive no MCP-sourced tools even when the parent Agent has MCP tools

#### Scenario: Recursive delegation refused
- **WHEN** a child model requests `delegate_skill` directly or indirectly
- **THEN** dispatch SHALL reject it and SHALL NOT create a grandchild attempt

### Requirement: Child permission principal
Each delegatable Utility SHALL use a stable child principal related to the stable parent Agent principal. Child actions SHALL be evaluated under explicit-Deny-first parent-chain ceilings, and a child Allow or remembered grant SHALL never override a Deny applicable to its parent.

#### Scenario: Parent Deny wins
- **WHEN** the child policy allows an action but the parent principal has an applicable explicit Deny
- **THEN** unified evaluation SHALL resolve the action to Deny without execution

#### Scenario: Child action asks independently
- **WHEN** delegation start was approved but a child write action has no resolving policy or grant
- **THEN** the system SHALL create a separate pending approval identifying the Utility child principal and action

#### Scenario: Stable principal reused
- **WHEN** the same parent Agent delegates the same canonical Utility in a later session
- **THEN** the system SHALL reuse the same stable child principal while creating a new delegation attempt

#### Scenario: Parent relationship cannot cycle
- **WHEN** a principal relationship would create a cycle or exceed the supported delegation depth
- **THEN** the system SHALL reject it without changing principal state

### Requirement: Bounded delegation lifecycle
The system SHALL enforce one active Utility child per parent generation, delegation depth one, and configured ceilings for total attempts, model/tool rounds, duration, input context, output, and evidence references. Limit exhaustion SHALL produce a structured terminal result and SHALL stop further child work.

#### Scenario: Second child requested while one is active
- **WHEN** the parent attempts another delegation before its active child reaches a terminal state
- **THEN** the system SHALL reject the second attempt as a concurrency-limit result

#### Scenario: Model round limit reached
- **WHEN** the child reaches its effective model/tool round ceiling without a final result
- **THEN** the attempt SHALL terminate as `limit_exceeded` and return bounded progress metadata

#### Scenario: Duration expires
- **WHEN** the child exceeds its effective duration ceiling
- **THEN** the runtime SHALL cancel provider and tool work and persist a timed-out terminal state

#### Scenario: Output exceeds limit
- **WHEN** the child final output exceeds the effective character limit
- **THEN** the system SHALL return a bounded prefix marked truncated and persist the original output only according to existing bounded message policy

### Requirement: Delegation cancellation and recovery
Stopping or ending a parent generation SHALL cancel its running or approval-blocked child work. Application restart SHALL not resume an in-flight child model generation automatically; it SHALL recover durable records to a terminal interrupted state.

#### Scenario: Parent generation stopped
- **WHEN** a user stops the parent while the child model or tool loop is active
- **THEN** the runtime SHALL cancel the child, resolve pending child approvals as stale, and return a cancelled delegation result

#### Scenario: Child cancelled directly
- **WHEN** a user cancels a visible child activity
- **THEN** the child SHALL stop while the parent generation remains active and receives the cancelled result

#### Scenario: Runtime restarts during delegation
- **WHEN** the application restarts with a persisted non-terminal delegation attempt
- **THEN** recovery SHALL mark it interrupted, clear non-durable waits, and SHALL NOT silently rerun model or tool calls

### Requirement: Structured delegation result
A terminal delegation SHALL return a bounded structured result containing attempt id, canonical Utility id and revision, status, summary, evidence references, effective limits, tool and approval counts, truncation, duration, and safe error information. It SHALL NOT return hidden reasoning, raw credentials, or an unbounded child transcript.

#### Scenario: Successful result
- **WHEN** a child completes successfully
- **THEN** the parent tool loop SHALL receive the structured result and MAY use its summary and evidence in the parent response

#### Scenario: Child failure
- **WHEN** a child fails provider, validation, permission, tool, timeout, or limit processing
- **THEN** the parent SHALL receive a terminal structured failure without converting the entire parent generation to failed automatically

#### Scenario: Evidence reference unavailable
- **WHEN** an evidence reference cannot be resolved safely
- **THEN** the result SHALL omit its content and retain a safe unavailable marker

### Requirement: Delegation persistence and Utility usage
The system SHALL persist delegation attempts, terminal results, effective Utility revision, limits, parent and child identities, timestamps, tool summaries, approval summaries, and execution links. It SHALL increment Utility `use_count` once when an approved child attempt actually begins, not when a call is previewed, denied, rejected, or cancelled before start.

#### Scenario: Approved child starts
- **WHEN** permission allows delegation and the first child model request begins
- **THEN** the system SHALL persist the running attempt and increment Utility use once

#### Scenario: Delegation denied before start
- **WHEN** start permission is denied or times out
- **THEN** the system SHALL persist the denied attempt outcome without incrementing Utility use

#### Scenario: History queried
- **WHEN** a user requests a Utility's delegation history
- **THEN** the system SHALL return bounded paginated attempt summaries without exposing hidden prompts, secrets, or unrestricted paths

### Requirement: Web runtime simulation
The Web/mock runtime SHALL simulate eligible, approval-blocked, running, tool-active, completed, denied, failed, limited, and cancelled Utility delegations through the same frontend contracts without making provider, process, filesystem, or native permission side effects.

#### Scenario: Mock delegation completes
- **WHEN** Web mode runs its deterministic eligible Utility scenario
- **THEN** it SHALL emit the same child lifecycle and structured-result shapes used by desktop mode

#### Scenario: Mock child approval
- **WHEN** a Web scenario reaches a child action requiring approval
- **THEN** it SHALL use the existing mock pending-approval contract with parent and Utility context

