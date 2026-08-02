## MODIFIED Requirements

### Requirement: Deleting a registered API agent
The system SHALL let a user delete a registered API Agent and its stored credential, SHALL reject the deletion without making any changes if the Agent is still referenced by blocking stored data, and SHALL remove every non-blocking Skill binding and Skill mount-path configuration owned by that Agent when deletion succeeds.

#### Scenario: Delete an unreferenced agent
- **WHEN** a user deletes an API Agent that has no sessions, messages, memories, usage records, or Loop worker/verifier assignments referencing it
- **THEN** the system SHALL remove the Agent and its stored credential
- **AND** SHALL remove its API Skill bindings, any legacy CLI Skill bindings, and its Skill mount-path configuration in the same transaction

#### Scenario: Delete rejected when the agent is still referenced
- **WHEN** a user deletes an API Agent that has at least one session, memory, usage record, or Loop worker/verifier assignment referencing it
- **THEN** the system SHALL reject the deletion
- **AND** SHALL report which kinds of data still reference the Agent
- **AND** SHALL NOT remove the Agent, its credential, Skill bindings, mount-path configuration, or any referencing data
