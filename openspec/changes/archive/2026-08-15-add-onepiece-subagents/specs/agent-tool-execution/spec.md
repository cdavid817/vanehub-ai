## ADDED Requirements

### Requirement: OnePiece child delegation eligibility
The native tool registry SHALL contain a fixed `delegate_subagent` handler that is eligible only for stable Agent id `onepiece`, only in execute mode, and only when the session has a canonical workspace and a ready active provider Profile. A user-created API Agent SHALL NOT acquire it by copying display name, provider metadata, or capability tags. Starting a child attempt SHALL be classified as its own delegation-start operation and SHALL default to explicit approval; that approval SHALL NOT grant the child any authority the parent session does not already have.

#### Scenario: OnePiece receives the tool
- **WHEN** a chat generation starts for stable Agent id `onepiece` in execute mode with a canonical workspace and a ready Profile
- **THEN** the outgoing provider request SHALL declare `delegate_subagent`

#### Scenario: Plan mode excludes it
- **WHEN** a generation starts in plan mode
- **THEN** the outgoing provider request SHALL exclude `delegate_subagent`

#### Scenario: Custom API Agent is denied
- **WHEN** a user-created API Agent with OnePiece-like metadata requests `delegate_subagent`
- **THEN** the registry SHALL deny eligibility because its stable id is not `onepiece`

#### Scenario: Starting a child requires approval
- **WHEN** OnePiece calls `delegate_subagent` and no policy resolves the action to `Allow` or `Deny`
- **THEN** the system SHALL request user approval before starting the child attempt

#### Scenario: Approval does not widen authority
- **WHEN** a child attempt is approved
- **THEN** the child's tool pool SHALL still be bounded by the parent session's own permission mode
