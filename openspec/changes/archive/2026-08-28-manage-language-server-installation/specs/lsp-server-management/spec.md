## ADDED Requirements

### Requirement: A language may declare a published distribution

A registered language MAY declare where its server is published: an allowlisted host, a URL, an integrity expectation, and extraction limits. The system SHALL acquire it through `managed-tool-installation` rather than through any download path of its own, and SHALL install it into a directory VaneHub owns.

#### Scenario: A declared server is installed

- **WHEN** a user installs a language that declares a distribution
- **THEN** the artifact SHALL be retrieved and extracted under the shared capability's bounds
- **AND** the finished install SHALL be placed only after extraction completes, so an interrupted install never leaves a directory that looks installed

#### Scenario: Installation is refused

- **WHEN** the shared retrieval or extraction refuses the artifact
- **THEN** the install SHALL fail with that reason
- **AND** no partially extracted directory SHALL remain, and the language SHALL still report as not installed

#### Scenario: A declared artifact publishes no digest

- **WHEN** a language declares a distribution with no published digest
- **THEN** the download SHALL still apply the host allowlist, the byte ceiling, the deadline, and cancellation
- **AND** the surface offering the install SHALL state that the bytes are not verified, rather than presenting an unverified download as a verified one

#### Scenario: A language declares no distribution

- **WHEN** a registered language declares no published distribution
- **THEN** no install action SHALL be offered for it
- **AND** its discovery SHALL behave exactly as it did before this capability existed

### Requirement: A managed install is removable and never confused with the user's own

The system SHALL remove only the install directory it created, SHALL leave a manually configured directory untouched, and SHALL report the language as not installed once its managed directory is gone.

#### Scenario: A managed install is removed

- **WHEN** a user uninstalls a language whose server VaneHub installed
- **THEN** the managed directory SHALL be removed
- **AND** any running server for that language SHALL be stopped first, because a running server holds files in that directory open

#### Scenario: The user pointed at their own copy

- **WHEN** a user uninstalls a language while a manual override names a directory VaneHub did not create
- **THEN** only the managed directory SHALL be removed
- **AND** the directory the override names SHALL be left exactly as it was

## MODIFIED Requirements

### Requirement: A manual override means what the launch shape says it means

A manual override for an executable-shaped language SHALL remain an absolute path to an executable file. A manual override for an interpreter-shaped language SHALL be an absolute path to the server's install directory, and SHALL be validated by the presence of the artifact its argument template requires rather than by executability. Where no override is configured and a managed install exists, discovery SHALL use the managed install; an override SHALL always take precedence over one.

#### Scenario: An interpreter-shaped override names a directory

- **WHEN** a user configures an absolute directory that contains the artifact the language's template requires
- **THEN** discovery SHALL report the server as available
- **AND** it SHALL report the resolved artifact rather than the directory alone

#### Scenario: An interpreter-shaped override names a directory without the artifact

- **WHEN** a configured directory exists but does not contain the required artifact
- **THEN** discovery SHALL report unavailable with a reason distinguishing this from a missing directory
- **AND** no server SHALL be started

#### Scenario: An interpreter-shaped language has no override

- **WHEN** an interpreter-shaped language is enabled with no configured install directory and no managed install
- **THEN** discovery SHALL report unavailable with a reason saying the install directory is not set
- **AND** it SHALL NOT search the executable path for the server, because the server is not an executable

#### Scenario: A managed install is used when no override is set

- **WHEN** an interpreter-shaped language has a managed install and no configured override
- **THEN** discovery SHALL resolve the server from the managed install
- **AND** it SHALL report the resolved artifact, so a reader can tell which version will run

#### Scenario: An override is set alongside a managed install

- **WHEN** both a manual override and a managed install exist for the same language
- **THEN** discovery SHALL use the override
- **AND** the managed install SHALL be left in place rather than removed, because the user may switch back
