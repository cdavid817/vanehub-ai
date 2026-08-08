## ADDED Requirements

### Requirement: Hook installation requires the wrapper binary to exist
VaneHub SHALL NOT write a permission-hook entry into Claude Code's global settings unless the wrapper binary named by that entry is present on disk. When it is absent, enabling hook management SHALL fail with an error identifying the expected location, and SHALL leave the user's Claude Code settings unmodified.

#### Scenario: Wrapper binary is absent
- **WHEN** hook management is enabled and the wrapper binary is not present at its resolved path
- **THEN** the operation SHALL fail with an error naming that path
- **AND** Claude Code's global settings SHALL NOT be modified

#### Scenario: Wrapper binary is present
- **WHEN** hook management is enabled and the wrapper binary exists at its resolved path
- **THEN** the hook entries SHALL be written as before

#### Scenario: Removing a hook installed by an earlier build
- **WHEN** hook management is disabled while the wrapper binary is absent
- **THEN** the removal SHALL still clear VaneHub's entries from Claude Code's settings

#### Scenario: Distribution does not carry the wrapper binary
- **WHEN** a packaged build does not ship the wrapper binary
- **THEN** the limitation SHALL be stated to users rather than surfacing as a hook Claude Code cannot execute
