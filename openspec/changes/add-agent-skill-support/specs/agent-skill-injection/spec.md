## ADDED Requirements

### Requirement: Skill binding to API agents
The system SHALL allow a registered API agent to bind to existing Skills through a non-mount binding that carries no filesystem path or mount state, distinct from the existing CLI mount-path binding mechanism.

#### Scenario: Bind a Skill to an API agent
- **WHEN** a user binds an existing Skill to a registered API agent
- **THEN** the system SHALL persist the binding without creating or requiring any filesystem mount

#### Scenario: Unbind a Skill from an API agent
- **WHEN** a user unbinds a Skill from an API agent
- **THEN** the system SHALL remove that binding without affecting the Skill's source, metadata, or any other binding

#### Scenario: Same Skill usable by both CLI and API agents
- **WHEN** a Skill already bound to one or more CLI agents via mount-path binding is also bound to an API agent
- **THEN** the system SHALL treat both bindings independently against the same underlying Skill content

### Requirement: System-prompt injection from bound Skills
The system SHALL inject the content of all bound and enabled Skills for an API agent into that agent's generation requests as a system prompt.

#### Scenario: Single bound Skill injected
- **WHEN** a generation runs for an API agent with exactly one bound, enabled Skill
- **THEN** the request SHALL include that Skill's content as the system prompt

#### Scenario: Multiple bound Skills concatenated deterministically
- **WHEN** a generation runs for an API agent with multiple bound, enabled Skills
- **THEN** the system SHALL concatenate their content into one system prompt in a deterministic order

#### Scenario: Disabled binding excluded
- **WHEN** a Skill is bound to an API agent but the binding is disabled
- **THEN** the system SHALL exclude that Skill's content from the system prompt

#### Scenario: No bound Skills means no system prompt
- **WHEN** an API agent has no bound, enabled Skills
- **THEN** the system SHALL send the request without a system prompt, unchanged from current behavior

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
- **THEN** the system SHALL log the failure and send the request without a system prompt

### Requirement: Web runtime parity
The Web/mock runtime SHALL expose equivalent Skill-to-API-agent binding behavior and a deterministic signal that bound Skills influenced a mock response.

#### Scenario: Web mock binding and injection
- **WHEN** a user binds a Skill to an API agent in Web/mock mode
- **THEN** the Web adapter SHALL persist the binding through the same mock event contract
- **AND** a subsequent mock generation for that agent SHALL deterministically signal that bound Skill content was applied, without calling a real provider
