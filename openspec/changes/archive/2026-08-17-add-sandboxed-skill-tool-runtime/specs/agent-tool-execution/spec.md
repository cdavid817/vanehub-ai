## ADDED Requirements

### Requirement: Contextual Skill tool catalog assembly
Native API agent generations SHALL receive only enabled Skill tool definitions applicable to their effective Skill set, active Role state, delegated Utility context, trust state, policy eligibility, and execution mode. Skill tools MUST NOT become globally visible merely because a package is installed.

#### Scenario: Role Skill is loaded in a session
- **WHEN** a Role Skill with eligible tools is active for a native API agent session
- **THEN** its eligible tool definitions are included in that session's subsequent generation requests

#### Scenario: Utility Skill is delegated
- **WHEN** a Utility Skill is invoked in an isolated delegated execution context
- **THEN** its eligible tools are exposed only inside that delegated context

#### Scenario: CLI-managed session is active
- **WHEN** a session is owned by an external CLI without a native Skill tool bridge
- **THEN** the system does not claim that locally registered Skill tools are available to that CLI

### Requirement: Skill tool dispatch and result validation
The agent tool loop SHALL dispatch canonical Skill tool ids to the Skill tool runtime and SHALL validate inputs before execution and outputs before returning them to the model. Unknown, stale, disabled, quarantined, or schema-invalid calls SHALL fail closed as bounded tool results.

#### Scenario: Valid Skill tool call
- **WHEN** the model calls an eligible Skill tool with schema-valid arguments
- **THEN** the runtime executes the pinned revision and returns a schema-valid bounded result

#### Scenario: Stale tool id is called
- **WHEN** a model calls a Skill tool id from a registry revision no longer valid for the generation context
- **THEN** the system rejects the call without falling back to another revision or similarly named tool

### Requirement: Skill tool calls remain visible and cancellable
Skill tool calls and their delegated host operations SHALL participate in the existing generation cancellation, lifecycle events, transcript persistence, and tool-call round-trip limits.

#### Scenario: Completed message used a Skill tool
- **WHEN** a generation completes after one or more Skill tool calls
- **THEN** the assistant message records each Skill tool call with its canonical id, provenance, status, and redacted result summary

#### Scenario: Round-trip limit is reached
- **WHEN** Skill and non-Skill tool calls together reach the generation's round-trip limit
- **THEN** the generation ends under the existing limit without granting an additional Skill-specific allowance

