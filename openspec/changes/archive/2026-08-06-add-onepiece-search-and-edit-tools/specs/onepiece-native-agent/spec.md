## MODIFIED Requirements

### Requirement: Safe OnePiece tool defaults
The system SHALL initialize and reset OnePiece with automatic shell, file-write, and file-edit approval disabled and SHALL continue applying the existing MCP approval and plan-mode restrictions. Read-only content-search and filename-search calls SHALL NOT require approval.

#### Scenario: First configuration retains approval prompts
- **WHEN** OnePiece is configured for the first time
- **THEN** shell, file-write, and file-edit calls SHALL require approval until the user explicitly enables the existing trust setting

#### Scenario: Read-only search does not prompt
- **WHEN** OnePiece requests a content-search or filename-search tool call
- **THEN** the system SHALL execute it without an approval prompt regardless of the trust setting

#### Scenario: Trust does not bypass existing hard gates
- **WHEN** a trusted OnePiece requests an MCP tool or runs in plan mode
- **THEN** the existing MCP approval and plan-mode restrictions SHALL remain in force
