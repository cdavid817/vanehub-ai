## ADDED Requirements

### Requirement: Delegation approval context
Pending approval presentation for a delegation start or child action SHALL identify the parent Agent, canonical Utility, effective Utility revision, delegated task summary, workspace scope, action, resource, risk, and effective capability ceiling without exposing hidden prompts or credentials.

#### Scenario: Delegation start approval displayed
- **WHEN** a Utility delegation start resolves to Ask
- **THEN** the approval UI SHALL distinguish it from an ordinary tool call and show the Utility, parent Agent, task summary, workspace, risk, and capability ceiling

#### Scenario: Child action approval displayed
- **WHEN** a child tool action resolves to Ask
- **THEN** the approval UI SHALL show both the Utility child principal and parent Agent plus the specific action and resource

#### Scenario: Remember start only
- **WHEN** a user approves delegation start with a remembered scope
- **THEN** the resulting grant SHALL apply to the delegation-start action and resource only
- **AND** SHALL NOT grant the Utility's child tool actions

### Requirement: Delegation approval lifecycle
Delegation approvals SHALL remain Rust-side authoritative and SHALL be linked to the parent generation and child attempt when one exists. Stopping the parent or child SHALL make related unresolved approvals stale and fail closed.

#### Scenario: Parent cancellation clears start approval
- **WHEN** the parent is cancelled while start approval is pending
- **THEN** the approval SHALL resolve as stale without starting a child

#### Scenario: Child cancellation clears action approval
- **WHEN** a child is cancelled while its tool action approval is pending
- **THEN** the approval SHALL resolve as stale and the action SHALL NOT execute

#### Scenario: Web approval parity
- **WHEN** Web/mock mode simulates delegation start or child-action approval
- **THEN** it SHALL use the same pending-list, event, scoped-decision, stale, and context contracts as desktop mode

