## MODIFIED Requirements

### Requirement: Risk-tiered tool approval
The system SHALL classify each tool call's risk by which tool/operation is being invoked, not by inspecting its specific arguments, and SHALL resolve whether it executes immediately, is denied, or requires approval through the unified permission evaluation defined by `permissions-core`. File-read operations SHALL execute without requiring user approval. File-write operations and shell execution SHALL, by default, require an explicit user approval before executing, unless the acting principal's assigned policy resolves the action to `Allow` or `Deny`.

#### Scenario: File read executes without approval
- **WHEN** the native agent calls the file tool with a read operation
- **THEN** the system SHALL execute it immediately without requesting user approval

#### Scenario: File write requires approval by default
- **WHEN** the native agent calls the file tool with a write operation and no policy resolves the action to `Allow` or `Deny`
- **THEN** the system SHALL request user approval before executing it, regardless of the file path or content involved

#### Scenario: Shell execution requires approval by default
- **WHEN** the native agent calls the shell tool and no policy resolves the action to `Allow` or `Deny`
- **THEN** the system SHALL request user approval before executing it, regardless of the specific command

#### Scenario: A policy-allowed file write or shell call executes without approval
- **WHEN** the acting principal's assigned policy resolves a file-write or shell-execution action to `Allow`
- **THEN** the system SHALL execute it immediately without requesting user approval

#### Scenario: A policy-denied file write or shell call is rejected without prompting
- **WHEN** the acting principal's assigned policy resolves a file-write or shell-execution action to `Deny`
- **THEN** the system SHALL NOT execute it
- **AND** SHALL NOT request user approval
