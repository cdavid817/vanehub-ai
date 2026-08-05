## MODIFIED Requirements

### Requirement: Web runtime terminal parity
The Web/mock runtime SHALL expose the same Agent Terminal service shape without claiming local CLI process execution.

#### Scenario: Web mock opens terminal
- **WHEN** the app runs in Web mode and an Agent Terminal is opened
- **THEN** the Web adapter SHALL provide deterministic simulated terminal state
- **AND** it SHALL NOT access local executables, SQLite, PowerShell, cmd, or a platform shell

#### Scenario: Web mock preserves session metadata
- **WHEN** the Web mock simulates terminal start or resume
- **THEN** it SHALL preserve session agent id, lifecycle state, and mock runtime session id behavior through the frontend service contract

#### Scenario: Web mock unavailable-CLI notice uses one locale
- **WHEN** the Agent Terminal displays the "local CLI execution unavailable in Web mode" notice
- **THEN** it SHALL render as a single line in the active application locale
- **AND** it SHALL NOT print both a Simplified Chinese and an English line for the same notice
