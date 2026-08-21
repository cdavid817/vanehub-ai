## ADDED Requirements

### Requirement: Shell natural-exit reclamation
The desktop shell runtime MUST remove and reap a managed PTY child after the child exits naturally, and frontend event subscriptions MUST remain cleanup-safe while registration is pending.

#### Scenario: Shell exits without an explicit disconnect
- **WHEN** a local PTY reaches EOF or reports process exit before the user requests disconnect
- **THEN** the runtime SHALL remove the matching shell generation from its live registry and wait for the child without affecting a replacement shell

#### Scenario: Shell view unmounts during subscription
- **WHEN** a Shell view is disposed before asynchronous event subscription completes
- **THEN** the completed subscription SHALL be immediately removed and SHALL NOT deliver events to the disposed terminal

