## ADDED Requirements

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
