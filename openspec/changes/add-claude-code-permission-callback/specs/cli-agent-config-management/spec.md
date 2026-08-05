## ADDED Requirements

### Requirement: Claude Code permission-hook projection
The system SHALL provide an operation, independent of profile/provider application, that installs or removes a single VaneHub-owned entry in the resolved Claude Code `settings.json`'s `hooks.PreToolUse` array, atomically, while preserving every other entry in `hooks` and all unrelated top-level fields.

#### Scenario: Install the permission hook
- **WHEN** the permissions capability requests the hook be installed for the `claude-code` principal
- **THEN** the system SHALL atomically write the VaneHub-owned `PreToolUse` entry into the resolved `settings.json`
- **AND** SHALL preserve every other hook entry and all unrelated top-level fields

#### Scenario: Remove the permission hook
- **WHEN** the permissions capability requests the hook be removed
- **THEN** the system SHALL atomically remove only the VaneHub-owned entry from `hooks.PreToolUse`
- **AND** SHALL leave every other hook entry and all unrelated top-level fields unchanged

#### Scenario: Hook projection is independent of profile application
- **WHEN** a Claude Code provider profile is applied, switched, or removed
- **THEN** the state of the VaneHub-owned `PreToolUse` entry SHALL NOT change as a side effect of that operation

#### Scenario: Existing configuration is malformed
- **WHEN** the existing Claude Code live file cannot be parsed safely
- **THEN** the system SHALL reject the hook projection with the resolved path and a user-actionable parse error
- **AND** SHALL leave the file unchanged

#### Scenario: Live file changes during projection
- **WHEN** the target live-file fingerprint changes after the projection plan is built but before replacement
- **THEN** the system SHALL abort with a drift conflict
- **AND** SHALL NOT overwrite the external edit
