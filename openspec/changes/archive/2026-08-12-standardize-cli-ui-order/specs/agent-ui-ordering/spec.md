## Purpose

Defines predictable Agent ordering across settings surfaces while retaining stable behavior for custom and future Agent entries.

## ADDED Requirements

### Requirement: Settings Agent choices use one shared priority
Every settings surface that presents CLI or Agent choices SHALL prioritize stable Agent ids in this order: `claude-code`, `codex-cli`, `opencode`, `antigravity-cli`, `gemini-cli`, then `onepiece`.

#### Scenario: Render all built-in choices in settings
- **WHEN** a settings surface receives all five managed CLI Agents and OnePiece in any source order
- **THEN** it SHALL display Claude Code, Codex CLI, OpenCode, Antigravity CLI, Gemini CLI, then OnePiece
- **AND** it SHALL preserve each entry's stable Agent id

#### Scenario: Render a subset of built-in choices
- **WHEN** a settings surface supports or receives only a subset of the prioritized Agents
- **THEN** it SHALL preserve the relative priority of the entries that are present
- **AND** it SHALL NOT synthesize unsupported Agent controls

### Requirement: Unrecognized Agents retain stable fallback order
Settings surfaces that include custom or future Agents SHALL place recognized priority entries first and SHALL preserve the source order among entries absent from the shared priority.

#### Scenario: Render custom Agents with built-in choices
- **WHEN** a settings surface receives recognized built-in choices followed by multiple custom or future Agents in a defined source order
- **THEN** it SHALL display the recognized choices using the shared priority
- **AND** it SHALL retain the custom or future Agents' relative source order after them

