# expert-role-management Specification

## Purpose
TBD - created by archiving change add-multi-agent-group-chat-session. Update Purpose after archive.
## Requirements
### Requirement: Expert role definition
The system SHALL store expert roles as reusable assets carrying a stable id, display name, avatar, colour, a one-line responsibility, a role instruction, optional Skill references, and peer-review eligibility.

#### Scenario: Create an expert role
- **WHEN** a user creates an expert role with a display name, responsibility, and role instruction
- **THEN** the system SHALL persist it with a stable id and make it available for seat assignment in every session
- **AND** the responsibility SHALL be required, because it is published to other Agents as the basis for choosing whom to hand off to

#### Scenario: Reject an incomplete role
- **WHEN** a user submits a role without a display name, responsibility, or role instruction
- **THEN** the system SHALL reject the submission with a localized validation message and SHALL NOT persist a partial role

#### Scenario: Edit a role already in use
- **WHEN** a user edits a role that is assigned to a seat in an existing session
- **THEN** the edit SHALL apply to sessions started afterwards
- **AND** running sessions SHALL retain the role text they were started with, so an in-flight conversation does not change identity mid-turn

### Requirement: Built-in starter roles
The system SHALL ship built-in expert roles that a user can use directly or copy and modify.

#### Scenario: Use a built-in role without authoring
- **WHEN** a user opens expert role management for the first time
- **THEN** the system SHALL present built-in roles covering at least architecture, code review, and implementation
- **AND** the user SHALL be able to assign one to a seat without writing any instruction text

#### Scenario: Copy a built-in role
- **WHEN** a user copies a built-in role
- **THEN** the system SHALL create an editable user-owned role seeded with the built-in content
- **AND** the built-in role SHALL remain unmodified

### Requirement: Peer review policy
An expert role SHALL declare whether it is eligible to be recommended as a peer reviewer and whether that recommendation requires a different model family.

#### Scenario: Role is not review-eligible
- **WHEN** a role does not declare peer-review eligibility
- **THEN** the system SHALL NOT recommend a seat holding that role as a reviewer

#### Scenario: Role requires a different model family
- **WHEN** a review-eligible role declares that review requires a different model family
- **THEN** seat assignment SHALL prefer Agents whose normalized model family differs from the Agent under review

### Requirement: Role instruction injection
The system SHALL inject a seat's role instruction through the Agent CLI's native system-prompt mechanism rather than prepending it to each user message.

#### Scenario: Role survives context compaction
- **WHEN** a seat's Agent runs long enough for its context to be compacted
- **THEN** the role instruction SHALL remain in effect, because it was injected through the CLI's native system-prompt channel
- **AND** the system SHALL NOT duplicate the role instruction into per-turn user messages

#### Scenario: Agent CLI exposes no native system-prompt channel
- **WHEN** a seat's Agent provides no native system-prompt mechanism
- **THEN** the system SHALL fall back to per-turn injection
- **AND** the seat SHALL surface that its role is not compaction-immune

