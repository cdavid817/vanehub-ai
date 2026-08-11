## ADDED Requirements

### Requirement: Successful Agent file mutations trigger targeted index reconciliation
The system SHALL publish the normalized relative path after a successful native Agent file write or scoped edit and SHALL asynchronously offer that path to the enabled code index for the same canonical workspace. Duplicate pending paths SHALL be coalesced, reconciliation SHALL preserve workspace-generation cancellation and existing admission rules, and notification failure SHALL NOT change the successful file-tool outcome.

#### Scenario: Agent edits an indexed source file
- **WHEN** a native Agent successfully edits a source file in an enabled code-index workspace
- **THEN** the code-index worker SHALL receive that normalized relative path for targeted reconciliation
- **AND** reconciliation SHALL remove or replace stale chunks, symbols, FTS entries, and vectors according to the current file content

#### Scenario: Agent writes in an unindexed workspace
- **WHEN** a native Agent successfully writes a file in a workspace whose code index is absent or disabled
- **THEN** the mutation notification SHALL perform no code-index work
- **AND** the file write SHALL remain successful

#### Scenario: Mutation queue already contains the path
- **WHEN** repeated successful edits publish the same workspace path before reconciliation begins
- **THEN** the background mutation queue SHALL coalesce the duplicate path without blocking the Agent tool thread

#### Scenario: Targeted reconciliation fails
- **WHEN** code-index storage or parsing fails after a successful Agent file mutation
- **THEN** the index SHALL record its existing safe degraded or audit state
- **AND** the completed file mutation SHALL NOT be retroactively changed into an Agent tool error
