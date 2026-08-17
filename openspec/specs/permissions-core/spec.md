# permissions-core Specification

## Purpose
TBD - created by archiving change add-permissions-core. Update Purpose after archive.
## Requirements
### Requirement: Unified permission decision model
The system SHALL evaluate every gated action, whether requested by a native API agent's tool-use loop or forwarded through the Claude Code permission-hook bridge, through a single decision point that resolves a `(principal, action, resource)` triple to exactly one of `Allow`, `Deny`, or `Ask`. A principal SHALL be identified by a stable agent id alone — one durable principal per agent, persisting across every session that agent participates in — with session id and generation id carried as per-evaluation context rather than as part of the principal's own identity.

#### Scenario: Evaluation produces one of three effects
- **WHEN** the native agent's tool-use loop requests an action requiring a permission decision
- **THEN** the system SHALL resolve it to exactly one of `Allow`, `Deny`, or `Ask` before the tool executes

#### Scenario: Unmatched action defaults to Ask
- **WHEN** no policy matches the requested principal, action, and resource
- **THEN** the system SHALL resolve the evaluation to `Ask`, not `Allow`

#### Scenario: The same principal is used across every session for an agent
- **WHEN** an agent participates in a new session it has never used before
- **THEN** the system SHALL evaluate that agent's actions against the same principal and policy assignment used in its other sessions, not a new, session-scoped principal

#### Scenario: CLI-originated evaluation uses the same decision point
- **WHEN** the Claude Code permission-hook bridge forwards a mapped tool call for the `claude-code` principal
- **THEN** the system SHALL resolve it through the same decision point, policy templates, and grants a native agent's equivalent action would use

### Requirement: Policy resolution order is explicit-Deny-first
The system SHALL resolve conflicting policy matches by giving explicit `Deny` priority over explicit `Allow`, and giving explicit `Allow` priority over the default `Ask`.

#### Scenario: Deny wins over a conflicting Allow
- **WHEN** two policies match the same principal, action, and resource with one resolving `Allow` and the other `Deny`
- **THEN** the system SHALL resolve the evaluation to `Deny`

### Requirement: Policy templates provide named, pre-built policy sets
The system SHALL provide four named policy templates — `readonly`, `standard`, `trusted`, and `yolo` — assignable per agent principal, where `standard` resolves `shell.exec` and `file.write` to `Ask`, `trusted` resolves both to `Allow`, `readonly` resolves both to `Deny`, and `yolo` resolves both to `Allow` while requiring a distinct confirmation step at assignment time.

#### Scenario: Trusted template auto-allows shell and file writes
- **WHEN** a principal is assigned the `trusted` template and requests `shell.exec` or `file.write`
- **THEN** the system SHALL resolve the evaluation to `Allow` without prompting

#### Scenario: Readonly template denies shell and file writes
- **WHEN** a principal is assigned the `readonly` template and requests `shell.exec` or `file.write`
- **THEN** the system SHALL resolve the evaluation to `Deny` without prompting

#### Scenario: Standard template asks for shell and file writes
- **WHEN** a principal is assigned the `standard` template and requests `shell.exec` or `file.write`
- **THEN** the system SHALL resolve the evaluation to `Ask`

### Requirement: MCP-sourced actions are floored at Ask regardless of template or policy
The system SHALL resolve every `mcp.tool` action to `Ask` before consulting any policy or template, and no template — including `yolo` — SHALL be able to produce `Allow` or `Deny` for an `mcp.tool` action.

#### Scenario: Trusted or yolo template still asks for an MCP call
- **WHEN** a principal assigned the `trusted` or `yolo` template requests an `mcp.tool` action
- **THEN** the system SHALL resolve the evaluation to `Ask`

### Requirement: Policy-denied actions skip execution without prompting
The system SHALL skip execution of an action whose evaluation resolves to `Deny` and report the denial as the tool's result, without presenting an approval prompt.

#### Scenario: Denied action does not execute and does not prompt
- **WHEN** an evaluation resolves an action to `Deny`
- **THEN** the system SHALL NOT execute that action and SHALL NOT create a pending approval request
- **AND** the system SHALL report the denial back to the provider as the tool's result, allowing the generation to continue

### Requirement: Remembered grants are consulted before falling back to templates
The system SHALL persist a remembered decision as a grant when an approval is resolved with a scope of `Session`, `Project`, or `Global`, and SHALL NOT persist a grant when resolved with a scope of `Once`. The system SHALL consult an unexpired, matching grant before evaluating templates and policies.

#### Scenario: Session-scoped grant is reused within the same session
- **WHEN** a principal's pending approval for an action and resource is resolved with `Scope: Session`
- **THEN** the system SHALL persist a grant covering that principal, action, and resource for the remainder of the session
- **AND** a subsequent identical evaluation within the same session SHALL resolve to the granted effect without creating a new pending approval

#### Scenario: Once-scoped resolution is not remembered
- **WHEN** a pending approval is resolved with `Scope: Once`
- **THEN** the system SHALL NOT persist a grant
- **AND** the next identical evaluation SHALL be evaluated again from templates and policies

### Requirement: Delegation fields are reserved but rejected until activated
The system SHALL persist `parent_principal_id` and `budget_config` columns on every principal record, and SHALL reject any attempt to set a non-null `parent_principal_id` with a `delegation_not_enabled` error until a future change activates delegation.

#### Scenario: Setting a parent principal is rejected
- **WHEN** a caller attempts to create or update a principal with a non-null `parent_principal_id`
- **THEN** the system SHALL reject the request with a `delegation_not_enabled` error
- **AND** SHALL NOT persist the parent relationship

### Requirement: Every decision is recorded in a full audit trail
The system SHALL record every evaluation's outcome — including the resolving principal, action, resource, effect, risk level, deciding mechanism, and channel — in an append-only audit record.

#### Scenario: Evaluation outcome is audited
- **WHEN** an evaluation resolves to `Allow`, `Deny`, or `Ask`
- **THEN** the system SHALL append an audit record identifying the principal, action, resource, resolved effect, and risk level

#### Scenario: Non-human decisions are attributed
- **WHEN** an approval resolves through timeout or is rejected as stale rather than through a human decision
- **THEN** the audit record SHALL identify the deciding mechanism as `timeout` or `stale_generation` respectively, not as a human decision

### Requirement: Evaluation failure fails closed
The system SHALL treat an internal failure to complete an evaluation (including a storage failure) as equivalent to `Ask`, and SHALL NOT execute the requested action as a result of that failure.

#### Scenario: Storage failure during evaluation does not auto-allow
- **WHEN** the system cannot complete an evaluation due to an internal or storage failure
- **THEN** the system SHALL NOT resolve the evaluation to `Allow`
- **AND** SHALL treat the action as requiring human approval or fail-closed denial

### Requirement: Legacy per-agent trust flag migrates to an equivalent policy assignment
The system SHALL, in a one-time migration, assign the `trusted` template to every agent principal whose legacy `auto_approve_tools` setting was enabled, and assign the `standard` template to every other existing agent principal, and SHALL NOT read or write the legacy setting afterward.

#### Scenario: Previously trusted agent keeps its effective behavior after migration
- **WHEN** an agent had `auto_approve_tools` enabled before this migration
- **THEN** after migration, that agent's principal SHALL resolve `shell.exec` and `file.write` to `Allow`
- **AND** SHALL still resolve `mcp.tool` to `Ask`
- **AND** Plan Mode SHALL still unconditionally block `shell.exec`, `file.write`, and `mcp.tool` for that agent

#### Scenario: Previously untrusted agent is unaffected by migration
- **WHEN** an agent did not have `auto_approve_tools` enabled before this migration
- **THEN** after migration, that agent's principal SHALL resolve `shell.exec` and `file.write` to `Ask`, matching its pre-migration behavior

### Requirement: Web runtime permission evaluation parity
The Web/mock runtime SHALL simulate template assignment, grant persistence, and evaluation outcomes through the same service contracts the desktop runtime uses, without performing real process execution or filesystem access.

#### Scenario: Mock evaluation follows the same effect contract
- **WHEN** a user exercises a simulated tool-call sequence in Web/mock mode
- **THEN** the Web adapter SHALL resolve each simulated action to `Allow`, `Deny`, or `Ask` through the same contract the desktop runtime uses

### Requirement: Newly created principals default to a configurable template
The system SHALL determine the policy template assigned to a newly created agent principal from a user-configurable default setting, falling back to `standard` when that setting is absent or unreadable.

#### Scenario: New agent inherits the configured default
- **WHEN** an agent principal is created for the first time and a default-template setting has been configured
- **THEN** the system SHALL assign that configured template to the new principal

#### Scenario: Missing or unreadable setting falls back to standard
- **WHEN** an agent principal is created for the first time and no default-template setting is configured, or it cannot be read
- **THEN** the system SHALL assign the `standard` template to the new principal

#### Scenario: Changing the default does not affect existing principals
- **WHEN** the default-template setting is changed after an agent principal already exists
- **THEN** the system SHALL NOT change that existing principal's already-assigned template

### Requirement: Skill tool execution principals
The permission system SHALL evaluate a Skill-contributed operation under a stable principal containing the parent agent principal, Skill id, tool id, effective revision hash, scope, workspace, session, and delegation context. It MUST NOT derive authorization from display names or Skill trust alone.

#### Scenario: Skill tool delegates a host operation
- **WHEN** a Skill tool requests an existing protected host operation
- **THEN** permission evaluation receives both the Skill tool principal and the requested resource and action

#### Scenario: Principal context is incomplete
- **WHEN** the runtime cannot establish the effective revision, session, workspace, or delegation provenance required by policy
- **THEN** permission evaluation fails closed before the operation executes

### Requirement: Capability declarations are upper bounds
A Skill manifest capability declaration SHALL constrain which operations may be requested but SHALL NOT imply Allow. The effective operation MUST satisfy the manifest declaration, runtime allowlist, execution mode, and unified permission decision.

#### Scenario: Policy allows undeclared capability
- **WHEN** policy would otherwise allow an operation that the Skill manifest did not declare
- **THEN** the runtime denies the operation without widening the manifest

#### Scenario: Manifest declares denied capability
- **WHEN** a manifest declares a capability that policy resolves to Deny
- **THEN** the operation is denied without prompting or executing

### Requirement: Recursive delegation is bounded
Permission evaluation SHALL preserve the complete bounded delegation chain and SHALL reject cycles or calls exceeding the configured Skill tool delegation depth.

#### Scenario: Tool delegation cycle is detected
- **WHEN** a Skill tool invocation would re-enter an ancestor tool in its delegation chain
- **THEN** the system rejects the call before further execution

### Requirement: Versioned Skill permission manifest
Every Skill-contributed tool SHALL declare requested host authority through a supported, normalized permission manifest containing separate filesystem read and write scopes, network origins, structured process commands, secret capability ids, and resource ceilings. The manifest SHALL be treated as an upper bound and MUST NOT create a grant, approval, or trust decision.

#### Scenario: Manifest requests workspace write access
- **WHEN** a tool declares write access to `workspace/src/**`
- **THEN** only a concrete canonical target matching that scope can proceed to independent permission evaluation

#### Scenario: Manifest contains an unsupported authority form
- **WHEN** a manifest contains an absolute path, parent traversal, shell command string, wildcard host, unknown secret id, unknown field, or unsupported version
- **THEN** validation fails closed before the tool becomes eligible

### Requirement: Provenance trust and authorization remain independent
The system SHALL classify Skill provenance as Built-in, Verified, Community, Local, or Untrusted and MAY use that classification to select a default policy. Provenance, package signature, checksum, or executable trust MUST NOT grant operational permission or reusable approval.

#### Scenario: Verified package requests a protected action
- **WHEN** a signature-verified Skill tool requests a protected filesystem, process, network, or secret action
- **THEN** the action receives the same concrete policy and approval evaluation required for an otherwise equivalent unverified principal

