# ssh-connection-management Specification
## Purpose
TBD - created by archiving change add-ssh-connection-management. Update Purpose after archive.
## Requirements
### Requirement: Durable SSH connection profiles
The system SHALL provide durable SSH connection profiles owned by the desktop runtime and mirrored by the Web/mock runtime.

#### Scenario: List SSH connections
- **WHEN** the frontend requests SSH connections through the service boundary
- **THEN** the system SHALL return bounded profile records with id, name, host, port, user, default path, authentication mode, credential presence, key path presence where applicable, test status, and timestamps
- **AND** the result SHALL NOT include stored password plaintext

#### Scenario: Create SSH connection
- **WHEN** a user creates an SSH connection with valid name, host, port, user, authentication mode, and default path
- **THEN** the system SHALL persist the non-secret profile metadata and return the created profile

#### Scenario: Reject invalid SSH connection
- **WHEN** an SSH connection mutation omits required fields, uses an invalid port, or provides an unsupported authentication mode
- **THEN** the system SHALL reject the mutation without persisting partial profile changes

### Requirement: SSH credential storage
The desktop runtime SHALL store SSH password credentials through native secure storage rather than SQLite or frontend state.

#### Scenario: Save password credential
- **WHEN** a desktop user saves an SSH connection with password authentication and a password value
- **THEN** the runtime SHALL store the password in native secure storage
- **AND** SQLite SHALL store only a credential reference and credential presence metadata

#### Scenario: Preserve existing password
- **WHEN** a desktop user edits an SSH connection with an existing password and submits no replacement password
- **THEN** the runtime SHALL preserve the existing native credential without returning the password to React

#### Scenario: Delete password credential
- **WHEN** a desktop user deletes an SSH connection or switches it away from password authentication
- **THEN** the runtime SHALL remove the associated native credential reference when one exists

#### Scenario: Web credential simulation
- **WHEN** the app runs in Web/mock mode and a user submits a password for an SSH connection
- **THEN** the Web adapter SHALL simulate credential presence without persisting or returning the password plaintext

### Requirement: SSH connection testing
The system SHALL test an SSH connection profile with a bounded TCP reachability probe to the configured SSH host and port.

#### Scenario: Test connection succeeds
- **WHEN** a user tests an SSH connection and the runtime can open a TCP connection to the configured SSH host and port within the timeout
- **THEN** the system SHALL mark the profile test status as succeeded
- **AND** it SHALL update the last connected timestamp

#### Scenario: Test connection fails
- **WHEN** a user tests an SSH connection and network reachability or timeout fails
- **THEN** the system SHALL mark the profile test status as failed
- **AND** it SHALL persist only a concise redacted failure summary

#### Scenario: Test does not authenticate or run remote commands
- **WHEN** the user starts an SSH connection test in the first-version implementation
- **THEN** the runtime SHALL NOT authenticate, run remote commands, or require an interactive SSH session

#### Scenario: Test does not launch interactive session
- **WHEN** the user starts an SSH connection test
- **THEN** the runtime SHALL NOT start an Agent CLI, interactive shell, remote file browser, or long-running remote process

### Requirement: SSH connection deletion preserves sessions
The system SHALL keep historical remote session metadata independent from SSH connection profile lifecycle.

#### Scenario: Delete connection with existing sessions
- **WHEN** a user deletes an SSH connection that was previously used to create remote sessions
- **THEN** the system SHALL delete the connection profile and associated credential
- **AND** existing sessions SHALL continue to expose their persisted remote workspace snapshot

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

### Requirement: SSH profile revisions
The desktop runtime SHALL increment a durable SSH profile revision whenever endpoint or authentication configuration changes.

#### Scenario: Edit connection configuration
- **WHEN** a user changes host, port, user, authentication mode, key path, or password credential
- **THEN** the profile revision SHALL increase and prior pooled connection keys SHALL become ineligible for new leases

#### Scenario: Rename profile
- **WHEN** a user changes only the display name
- **THEN** the runtime SHALL preserve the revision because endpoint and authentication compatibility did not change

### Requirement: SSH runtime authentication
The desktop runtime SHALL authenticate remote Terminal operations with the selected profile and native-owned credential material.

#### Scenario: Authenticate password profile
- **WHEN** a password-authenticated profile with a stored credential opens a remote operation
- **THEN** the SSH adapter SHALL retrieve the credential inside the native runtime and SHALL NOT return it to React or SQLite

#### Scenario: Authenticate key profile
- **WHEN** a key-authenticated profile with a valid key path opens a remote operation
- **THEN** the SSH adapter SHALL load the key through the native boundary and SHALL NOT persist private-key contents

#### Scenario: Missing authentication material
- **WHEN** a profile lacks required credential or usable key material
- **THEN** the runtime SHALL reject authentication before opening a remote PTY or exec channel

### Requirement: SSH host trust records
The desktop runtime SHALL store bounded host-key trust metadata separately from password credentials.

#### Scenario: Save confirmed trust
- **WHEN** a user confirms a first-seen host fingerprint
- **THEN** the runtime SHALL persist endpoint, algorithm, fingerprint, and confirmation timestamp without storing a raw authentication payload

#### Scenario: Delete profile trust
- **WHEN** an SSH connection profile is deleted
- **THEN** profile-scoped host trust metadata SHALL be removed unless another explicit trust owner retains it

