# agent-lifecycle-management Specification

## Purpose
TBD - created by archiving change add-agent-lifecycle-management. Update Purpose after archive.
## Requirements
### Requirement: Editing a registered API agent
The system SHALL let a user update a registered API agent's display name, model id, base URL, and stored API key without changing its id, provider, or interface format.

#### Scenario: Display name, model, or base URL edited
- **WHEN** a user submits new values for an existing API agent's display name, model id, and/or base URL
- **THEN** the system SHALL persist the new values against the same agent id
- **AND** the agent's provider and interface format SHALL remain unchanged

#### Scenario: API key rotated
- **WHEN** a user submits a new API key for an existing API agent
- **THEN** the system SHALL replace the stored credential with the new value
- **AND** subsequent generations for that agent SHALL use the new key

#### Scenario: Edit re-validates like registration
- **WHEN** a user submits an edit that omits a required base URL for an agent whose interface format is openai-compatible
- **THEN** the system SHALL reject the edit with a validation error
- **AND** SHALL NOT persist any part of the edit

#### Scenario: Provider and interface format are immutable
- **WHEN** a user attempts to change an existing API agent's provider or interface format through the edit operation
- **THEN** the system SHALL NOT apply that change

### Requirement: Deleting a registered API agent
The system SHALL let a user delete a registered API agent and its stored credential, and SHALL reject the deletion without making any changes if the agent is still referenced by other stored data.

#### Scenario: Delete an unreferenced agent
- **WHEN** a user deletes an API agent that has no sessions, messages, memories, usage records, or Loop worker/verifier assignments referencing it
- **THEN** the system SHALL remove the agent and its stored credential
- **AND** SHALL remove any Skill-to-agent bindings for that agent

#### Scenario: Delete rejected when the agent is still referenced
- **WHEN** a user deletes an API agent that has at least one session, memory, usage record, or Loop worker/verifier assignment referencing it
- **THEN** the system SHALL reject the deletion
- **AND** SHALL report which kinds of data still reference the agent
- **AND** SHALL NOT remove the agent, its credential, or any of the referencing data

### Requirement: Web runtime lifecycle-management parity
The Web/mock runtime SHALL simulate editing and deleting a registered API agent through the same service contracts the desktop runtime uses, including rejecting deletion of a referenced mock agent.

#### Scenario: Mock edit and delete
- **WHEN** a user edits or deletes a registered API agent in Web/mock mode
- **THEN** the Web adapter SHALL apply the same change to its in-memory agent registry
- **AND** SHALL enforce the same referenced-agent delete rejection the desktop runtime enforces

