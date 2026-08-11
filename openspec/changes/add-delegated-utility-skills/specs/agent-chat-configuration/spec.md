## ADDED Requirements

### Requirement: Permission mode constrains Utility delegation
The parent generation's permission mode SHALL be a non-bypassable ceiling on Utility eligibility and child tools. Plan mode SHALL permit delegation only when the Utility's effective capability set is entirely read-only and Plan-compatible.

#### Scenario: Read-only Utility in Plan mode
- **WHEN** a Plan-mode parent delegates an eligible Utility whose effective capabilities are entirely Plan-compatible and read-only
- **THEN** delegation MAY proceed through start permission evaluation with only read-only child tools

#### Scenario: Mutating Utility in Plan mode
- **WHEN** a Plan-mode parent targets a Utility whose declared or effective capabilities include shell, write, edit, MCP, or another non-Plan operation
- **THEN** the system SHALL refuse delegation before creating a child attempt

#### Scenario: Permission mode changes after start
- **WHEN** the user changes session configuration after a child attempt begins
- **THEN** the running child SHALL retain the stricter of its captured ceiling and any newly applied cancellation or deny action
- **AND** SHALL never gain capabilities from the configuration change

