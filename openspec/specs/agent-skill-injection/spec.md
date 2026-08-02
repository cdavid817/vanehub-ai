# agent-skill-injection Specification

## Purpose
TBD - created by archiving change add-agent-skill-support. Update Purpose after archive.
## Requirements
### Requirement: Skill binding to API agents
The system SHALL allow a registered API Agent to bind to existing Skills through a non-mount binding that carries no filesystem path or mount state, distinct from the CLI mount-path binding mechanism, and SHALL reject API binding requests for non-API Agents.

#### Scenario: Bind a Skill to an API agent
- **WHEN** a user binds an existing Skill to a registered API Agent
- **THEN** the system SHALL persist the binding without creating or requiring any filesystem mount

#### Scenario: Reject API binding to CLI agent
- **WHEN** a caller attempts to create an API prompt binding for a CLI-only Agent
- **THEN** the system SHALL reject it without changing binding state

#### Scenario: Unbind a Skill from an API agent
- **WHEN** a user unbinds a Skill from an API Agent
- **THEN** the system SHALL remove that binding without affecting the Skill's source, metadata, or any other binding

#### Scenario: Same Skill usable by both CLI and API agents
- **WHEN** a Skill already bound to one or more CLI Agents via mount-path binding is also bound to an API Agent
- **THEN** the system SHALL treat both bindings independently against the same underlying Skill content

### Requirement: System-prompt injection from bound Skills
The system SHALL inject enabled global Skills and enabled Skills bound for the active canonical workspace into an API Agent generation request, in deterministic order and within the configured Skill prompt budget.

#### Scenario: Single bound Skill injected
- **WHEN** a generation runs for an API Agent with exactly one applicable bound and enabled Skill
- **THEN** the request SHALL include that Skill's content as the Skill section of the system prompt

#### Scenario: Workspace Skill isolation
- **WHEN** an API Agent has Workspace Skill bindings from multiple projects
- **THEN** a generation SHALL include only the Workspace Skills matching the active session workspace, in addition to applicable global Skills

#### Scenario: Multiple bound Skills concatenated deterministically
- **WHEN** a generation has multiple applicable bound and enabled Skills
- **THEN** the system SHALL concatenate their content in deterministic scope, workspace, and Skill-id order

#### Scenario: Disabled Skill excluded
- **WHEN** a Skill is bound to an API Agent but the Skill is disabled
- **THEN** the system SHALL exclude that Skill's content from the system prompt

#### Scenario: Unreadable Skill does not suppress healthy Skills
- **WHEN** one applicable bound Skill cannot be read but other applicable Skills are healthy
- **THEN** the system SHALL log the failed Skill, omit it, and inject the healthy Skills

#### Scenario: No bound Skills means no Skill section
- **WHEN** an API Agent has no applicable bound and enabled Skills
- **THEN** the system SHALL send the request without a Skill section, unchanged from current behavior apart from any independently assembled memory section

### Requirement: Bounded Skill prompt assembly
The system SHALL limit each injected Skill to 8,000 characters and all injected Skills together to 16,000 characters without partially truncating a Skill instruction body.

#### Scenario: Skill exceeds individual budget
- **WHEN** a Skill body exceeds the individual character budget
- **THEN** the system SHALL skip it and write a warning through the unified logging boundary

#### Scenario: Aggregate budget exhausted
- **WHEN** the next deterministically ordered Skill would exceed the remaining aggregate budget
- **THEN** the system SHALL skip it, continue evaluating later smaller Skills, and log the omission

### Requirement: Provider-native system prompt placement
The system SHALL place the assembled system prompt using each wire format's native mechanism rather than a synthetic user-role message.

#### Scenario: Anthropic wire format
- **WHEN** an API agent using the Anthropic wire format has a non-empty system prompt
- **THEN** the system SHALL set it as the request's top-level `system` field

#### Scenario: OpenAI-compatible wire format
- **WHEN** an API agent using the OpenAI-compatible wire format has a non-empty system prompt
- **THEN** the system SHALL prepend it as a `role: "system"` message ahead of the conversation messages in the request

### Requirement: System prompt is immune to compaction
The system SHALL keep the assembled system prompt outside the turns list that context compaction measures and rewrites.

#### Scenario: Compaction does not alter or remove the system prompt
- **WHEN** context compaction triggers during a generation for an API agent with bound Skills
- **THEN** the system prompt SHALL remain present, complete, and unchanged on every subsequent request of that generation, including the summarization call itself

### Requirement: Graceful degradation on Skill lookup failure
The system SHALL proceed with generation when Skill lookup fails rather than failing the generation.

#### Scenario: Skill lookup fails
- **WHEN** looking up an API agent's bound Skills fails
- **THEN** the system SHALL log the failure and send the request without a Skill section while preserving any independently assembled system-prompt sections

### Requirement: Web runtime parity
The Web/mock runtime SHALL expose equivalent Skill-to-API-agent binding behavior and a deterministic signal that bound Skills influenced a mock response.

#### Scenario: Web mock binding and injection
- **WHEN** a user binds a Skill to an API agent in Web/mock mode
- **THEN** the Web adapter SHALL persist the binding through the same mock event contract
- **AND** a subsequent mock generation for that agent SHALL deterministically signal that bound Skill content was applied, without calling a real provider

