## MODIFIED Requirements

### Requirement: Guarded external command execution
The native runtime SHALL execute external commands only through backend-owned command construction or validated user configuration without shell string interpolation. Command construction SHALL NOT permit the operating system to attach interactive console UI to a child process, so a launch failure stays on the application's own error path.

#### Scenario: Backend-owned command
- **WHEN** the native runtime launches a known Agent or SDK command from backend-owned metadata
- **THEN** it SHALL construct the process invocation with explicit executable and argument values

#### Scenario: User-configured command
- **WHEN** the native runtime runs a user-configured MCP command
- **THEN** it SHALL validate the command configuration, avoid shell string interpolation, and record an audit log entry for the execution attempt

#### Scenario: Console-subsystem child is launched
- **WHEN** the native runtime constructs a command for a console-subsystem executable
- **THEN** the child SHALL NOT be given a console window

#### Scenario: A launched command fails to start
- **WHEN** launching an external command fails
- **THEN** the failure SHALL be returned to the calling native code as a handled error
- **AND** no operating-system component SHALL present a dialog the application cannot dismiss or record

#### Scenario: Capability detection runs at startup
- **WHEN** startup detection probes for CLI availability
- **THEN** those probes SHALL NOT make windows appear on the user's desktop
