## ADDED Requirements

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
