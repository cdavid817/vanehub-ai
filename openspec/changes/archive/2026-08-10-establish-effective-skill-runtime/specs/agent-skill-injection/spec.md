## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Delivery-aware compatibility
Existing Skills that do not declare type or delivery SHALL retain their prior eager API-agent behavior, while explicitly on-demand Role Skills SHALL rely on agent-initiated loading.

#### Scenario: Existing binding after upgrade
- **WHEN** an existing bound Skill omits type and delivery metadata and was injected before the upgrade
- **THEN** it SHALL remain eligible for eager injection after migration if it is otherwise enabled, available, and effective

#### Scenario: Explicit conversion to on-demand
- **WHEN** a mutable Skill is updated to valid `type: role` and `delivery: on-demand`
- **THEN** subsequent generations SHALL stop eagerly injecting it and SHALL expose it through Skill discovery and loading tools

