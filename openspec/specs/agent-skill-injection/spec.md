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
The system SHALL inject only enabled, available, effective Role Skills with `eager` delivery that apply to the active API Agent and canonical workspace. Each Skill section MUST NOT exceed 8,000 Unicode characters, all included Skill sections together MUST NOT exceed 16,000 Unicode characters, and selection within those limits SHALL follow deterministic effective-layer, workspace, and canonical Skill-id order. On-demand Role Skills and Utility Skills SHALL NOT be eagerly injected.

#### Scenario: Single bound Skill injected
- **WHEN** a generation runs for an API agent with exactly one bound, enabled, available, effective eager Role Skill within the per-item and aggregate budgets
- **THEN** the request SHALL include that Skill's content in the Skill section of the system prompt

#### Scenario: Workspace Skill isolation
- **WHEN** an API Agent has Workspace Skill bindings from multiple projects
- **THEN** a generation SHALL include only effective eager Role Skills matching the active canonical session workspace, in addition to applicable non-project Skills

#### Scenario: Multiple bound Skills concatenated deterministically
- **WHEN** a generation runs for an API agent with multiple applicable eager Role Skills that fit the aggregate budget
- **THEN** the system SHALL concatenate their content in deterministic effective-layer, workspace, and canonical Skill-id order

#### Scenario: Disabled Skill excluded
- **WHEN** an effective Skill is bound to an API Agent but is disabled
- **THEN** the system SHALL exclude that Skill's content from the system prompt

#### Scenario: Shadowed Skill excluded
- **WHEN** a bound Skill definition is shadowed by a higher-priority definition with the same canonical id
- **THEN** the system SHALL exclude the shadowed content and evaluate only the effective definition

#### Scenario: On-demand Role excluded
- **WHEN** an enabled Role Skill uses on-demand delivery
- **THEN** the system SHALL exclude its instructions from eager system-prompt assembly
- **AND** SHALL leave it discoverable through the fixed Skill tools

#### Scenario: Utility Skill excluded
- **WHEN** an enabled Skill is classified as Utility
- **THEN** the system SHALL exclude its instructions from eager system-prompt assembly regardless of its declared delivery

#### Scenario: Unreadable Skill does not suppress healthy Skills
- **WHEN** one applicable bound Skill cannot be read but other applicable Skills are healthy
- **THEN** the system SHALL log safe failure metadata, omit the unreadable Skill, and inject the healthy Skills

#### Scenario: Oversized individual Skill excluded
- **WHEN** an applicable eager Role Skill exceeds 8,000 Unicode characters
- **THEN** the system SHALL omit that Skill as a whole
- **AND** it SHALL write a warning containing safe Skill identity and size metadata without logging its body

#### Scenario: Aggregate Skill budget reached
- **WHEN** including the next deterministically ordered Skill would exceed the 16,000-character aggregate Skill budget
- **THEN** the system SHALL omit that Skill and continue evaluating later Skills that may fit the remaining budget
- **AND** included Skill order SHALL remain deterministic

#### Scenario: No bound Skills means no Skill section
- **WHEN** an API Agent has no applicable bound, enabled, available, effective eager Role Skills
- **THEN** the system SHALL send the request without a Skill section, unchanged apart from independently assembled core-instruction or memory sections and the fixed tool catalog

#### Scenario: No prompt sources means no system prompt
- **WHEN** an API Agent has no core instructions, no applicable eager Role Skills, and no scoped memories
- **THEN** the system SHALL send the request without a system prompt, while still allowing independently declared tools

#### Scenario: No bound Skills for OnePiece
- **WHEN** OnePiece has no applicable eager Role Skills
- **THEN** the system SHALL omit only the Skill section
- **AND** it SHALL retain its core instructions, any scoped memory section, and applicable fixed tools

### Requirement: Bounded Skill prompt assembly
The system SHALL limit each injected eager Role Skill to 8,000 Unicode characters and all injected Skills together to 16,000 Unicode characters without partially truncating a Skill instruction body. Prompt assembly SHALL use only effective Skill content and SHALL record use activity separately from that content.

#### Scenario: Skill exceeds individual budget
- **WHEN** an applicable eager Skill body exceeds the individual character budget
- **THEN** the system SHALL skip it and write a warning through the unified logging boundary

#### Scenario: Aggregate budget exhausted
- **WHEN** the next deterministically ordered eager Skill would exceed the remaining aggregate budget
- **THEN** the system SHALL skip it, continue evaluating later smaller Skills, and log the omission

#### Scenario: Included Skill usage tracked
- **WHEN** an eager Role Skill is included in the final system prompt
- **THEN** the system SHALL record one use for that Skill and generation without changing the instruction body

### Requirement: Delivery-aware compatibility
Existing Skills that do not declare type or delivery SHALL retain their prior eager API-agent behavior, while explicitly on-demand Role Skills SHALL rely on agent-initiated loading.

#### Scenario: Existing binding after upgrade
- **WHEN** an existing bound Skill omits type and delivery metadata and was injected before the upgrade
- **THEN** it SHALL remain eligible for eager injection after migration if it is otherwise enabled, available, and effective

#### Scenario: Explicit conversion to on-demand
- **WHEN** a mutable Skill is updated to valid `type: role` and `delivery: on-demand`
- **THEN** subsequent generations SHALL stop eagerly injecting it and SHALL expose it through Skill discovery and loading tools

### Requirement: Provider-native system prompt placement
The system SHALL place the assembled system prompt using each wire format's native mechanism rather than a synthetic user-role message.

#### Scenario: Anthropic wire format
- **WHEN** an API agent using the Anthropic wire format has a non-empty system prompt
- **THEN** the system SHALL set it as the request's top-level `system` field

#### Scenario: OpenAI-compatible wire format
- **WHEN** an API agent using the OpenAI-compatible wire format has a non-empty system prompt
- **THEN** the system SHALL prepend it as a `role: "system"` message ahead of the conversation messages in the request

### Requirement: System prompt is immune to compaction
The system SHALL keep the assembled system prompt, including Agent core instructions, custom instructions, Skill content, and scoped memories, outside the turns list that context compaction measures and rewrites.

#### Scenario: Compaction does not alter or remove the system prompt
- **WHEN** context compaction triggers during a generation with any assembled system-prompt sections
- **THEN** every included section SHALL remain present, complete, and unchanged on every subsequent request of that generation, including the summarization call itself

### Requirement: Graceful degradation on Skill lookup failure
The system SHALL proceed with generation when Skill lookup fails rather than failing the generation, and SHALL preserve independently resolved core-instruction and memory sections.

#### Scenario: Skill lookup fails
- **WHEN** looking up an ordinary API Agent's bound Skills fails
- **THEN** the system SHALL log the failure and omit the Skill section
- **AND** it SHALL send any independently resolved memory section

#### Scenario: Skill lookup fails for OnePiece
- **WHEN** looking up OnePiece's bound Skills fails
- **THEN** the system SHALL log the failure and omit the Skill section
- **AND** it SHALL retain the complete OnePiece core instructions and any independently resolved memory section

### Requirement: Web runtime parity
The Web/mock runtime SHALL expose equivalent Skill-to-API-agent binding behavior and a deterministic signal that bound Skills influenced a mock response.

#### Scenario: Web mock binding and injection
- **WHEN** a user binds a Skill to an API agent in Web/mock mode
- **THEN** the Web adapter SHALL persist the binding through the same mock event contract
- **AND** a subsequent mock generation for that agent SHALL deterministically signal that bound Skill content was applied, without calling a real provider

### Requirement: Deterministic API system-prompt section ordering
The system SHALL assemble an API Agent's provider-native system prompt from independently resolved sections in this order: mandatory Agent core instructions when defined, host-level custom instructions when enabled and non-empty, bound and enabled Skills, then scoped memories.

#### Scenario: Assemble all OnePiece prompt sources
- **WHEN** OnePiece has core instructions, enabled non-empty custom instructions, one or more included Skills, and scoped memories
- **THEN** the provider-native system prompt SHALL contain four distinctly delimited sections in core, custom-instructions, Skill, then memory order

#### Scenario: Optional section is empty
- **WHEN** an optional custom-instructions, Skill, or memory section resolves to no content
- **THEN** the system SHALL omit only that section without changing the order or content of the remaining sections

### Requirement: Overlay-applied instruction consumption
Every API Agent Skill instruction consumer SHALL use content produced by successful trusted Overlay replay for the active workspace context. It SHALL NOT inject untrusted, pinned-after mutation, invalid, or unresolved conflicted Overlay changes.

#### Scenario: Healthy Overlay affects eager instructions
- **WHEN** an applicable eager Skill has a healthy trusted Overlay
- **THEN** prompt assembly SHALL use the Overlay-applied effective instructions within the existing per-Skill and aggregate budgets

#### Scenario: Healthy Overlay affects on-demand load
- **WHEN** an applicable on-demand Role Skill has a healthy trusted Overlay
- **THEN** its load result SHALL use the same Overlay-applied effective instructions and resource view

#### Scenario: Untrusted Overlay excluded
- **WHEN** an applicable imported Overlay has not been promoted to trusted
- **THEN** agent-visible instructions and resources SHALL remain unchanged by that Overlay

#### Scenario: Conflicted Overlay falls back safely
- **WHEN** an applicable Overlay has unresolved replay conflicts
- **THEN** agent-visible content SHALL exclude that Overlay scope and use the last deterministic lower-scope or base content
- **AND** the system SHALL report a redacted warning through unified logging

