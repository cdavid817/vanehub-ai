## ADDED Requirements

### Requirement: SSH credential mutation consistency
The desktop runtime SHALL compensate secure-storage changes when the corresponding SSH profile metadata mutation cannot complete.

#### Scenario: Password profile update fails
- **WHEN** a profile update writes a new password credential and the SQLite profile update fails
- **THEN** the runtime SHALL restore the prior credential when one existed or remove the newly-created credential otherwise
- **AND** it SHALL return the profile update failure without persisting partial profile metadata

#### Scenario: Credential deletion fails during profile deletion
- **WHEN** SQLite profile deletion succeeds but native credential deletion fails
- **THEN** the runtime SHALL restore the profile metadata so the credential remains reachable for a later retry
- **AND** it SHALL report the deletion failure
