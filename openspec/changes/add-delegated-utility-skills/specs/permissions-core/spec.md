## MODIFIED Requirements

### Requirement: Delegation fields are reserved but rejected until activated
The system SHALL persist `parent_principal_id` and `budget_config` on every principal record. It SHALL permit a non-null parent only for a validated Utility child principal created by the native delegation service, SHALL reject unsupported callers and invalid graphs, and SHALL apply explicit-Deny-first evaluation across the child and parent chain.

#### Scenario: Setting a parent principal is rejected
- **WHEN** a caller outside the native Utility delegation service attempts to create or update a principal with a non-null `parent_principal_id`
- **THEN** the system SHALL reject the request with a delegation-not-authorized error
- **AND** SHALL NOT persist the parent relationship

#### Scenario: Valid Utility child principal
- **WHEN** the native delegation service creates or resolves the stable principal for an eligible Utility and parent Agent
- **THEN** the system SHALL persist or reuse the acyclic parent relationship and bounded budget configuration

#### Scenario: Parent explicit Deny constrains child
- **WHEN** a child evaluation would otherwise resolve to Allow but an applicable parent-chain policy resolves to explicit Deny
- **THEN** the final evaluation SHALL be Deny
- **AND** the audit record SHALL identify the parent ceiling as the deciding mechanism

#### Scenario: Invalid delegation graph
- **WHEN** a requested parent relationship creates a cycle, exceeds supported depth one, or points to a non-Agent parent principal
- **THEN** the system SHALL reject it without changing the existing principal graph

