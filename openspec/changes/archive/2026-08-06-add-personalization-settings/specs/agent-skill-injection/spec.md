# agent-skill-injection Specification (Delta)

## MODIFIED Requirements

### Requirement: Deterministic API system-prompt section ordering
The system SHALL assemble an API Agent's provider-native system prompt from independently resolved sections in this order: mandatory Agent core instructions when defined, host-level custom instructions when enabled and non-empty, bound and enabled Skills, then scoped memories.

#### Scenario: Assemble all OnePiece prompt sources
- **WHEN** OnePiece has core instructions, enabled non-empty custom instructions, one or more included Skills, and scoped memories
- **THEN** the provider-native system prompt SHALL contain four distinctly delimited sections in core, custom-instructions, Skill, then memory order

#### Scenario: Optional section is empty
- **WHEN** an optional custom-instructions, Skill, or memory section resolves to no content
- **THEN** the system SHALL omit only that section without changing the order or content of the remaining sections

### Requirement: System prompt is immune to compaction
The system SHALL keep the assembled system prompt, including Agent core instructions, custom instructions, Skill content, and scoped memories, outside the turns list that context compaction measures and rewrites.

#### Scenario: Compaction does not alter or remove the system prompt
- **WHEN** context compaction triggers during a generation with any assembled system-prompt sections
- **THEN** every included section SHALL remain present, complete, and unchanged on every subsequent request of that generation, including the summarization call itself
