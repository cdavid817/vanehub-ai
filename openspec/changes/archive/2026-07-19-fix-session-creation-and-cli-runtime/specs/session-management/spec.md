## ADDED Requirements

### Requirement: New session defaults

The system SHALL generate a default new-session name from the selected/current project folder basename followed by a timestamp.

#### Scenario: Default name uses folder and timestamp

- **WHEN** a user opens the create-session flow for `D:\work\demo-app`
- **THEN** the default session name SHALL start with `demo-app-`
- **AND** the suffix SHALL be a timestamp suitable for distinguishing sessions.

### Requirement: User-safe session path display

The system SHALL strip Windows extended-length path prefixes from displayed paths and from values used only for display-derived labels.

#### Scenario: Extended-length path is displayed normally

- **WHEN** the selected folder is `\\?\D:\cdavid\Documents\code\claude-code`
- **THEN** the UI SHALL display `D:\cdavid\Documents\code\claude-code`
- **AND** the default session name SHALL use `claude-code` as the folder basename.

#### Scenario: Project grouping displays normal paths

- **WHEN** a listed session folder is stored as `\\?\D:\cdavid\Documents\code\claude-code`
- **THEN** project grouping labels SHALL display `D:\cdavid\Documents\code\claude-code`.

### Requirement: Recent project selection

The create-session local project section SHALL label persisted project choices as recently opened projects.

#### Scenario: Recent projects are listed

- **WHEN** known local projects are available during session creation
- **THEN** the create-session page SHALL present them under a recently opened projects label.
