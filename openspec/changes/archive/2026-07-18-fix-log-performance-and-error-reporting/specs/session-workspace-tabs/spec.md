## ADDED Requirements

### Requirement: Desktop session-workspace command availability
The desktop runtime SHALL register declared session-workspace and shell commands that implement the frontend session-workspace service contract.

#### Scenario: Invoke a session-workspace operation in desktop mode
- **WHEN** the Tauri session-workspace adapter invokes a declared directory, document, Git, log, or shell operation
- **THEN** the desktop runtime SHALL route the command to its Rust implementation
- **AND** it SHALL return the documented service result rather than an unknown-command error

#### Scenario: Run session workspace in Web mode
- **WHEN** the session workspace runs through the Web/mock adapter
- **THEN** it SHALL retain the existing Web-compatible service behavior without requiring native command registration
