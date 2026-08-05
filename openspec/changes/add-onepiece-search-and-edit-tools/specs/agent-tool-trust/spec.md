## RENAMED Requirements

- FROM: `### Requirement: A trusted agent's shell and file-write calls skip approval`
- TO: `### Requirement: A trusted agent's shell, file-write, and edit calls skip approval`

## MODIFIED Requirements

### Requirement: A trusted agent's shell, file-write, and edit calls skip approval
The system SHALL execute a trusted native API agent's `shell` calls, file tool `write` operations, and file-edit calls immediately, without prompting for approval, while leaving every other tool's approval behavior unchanged.

#### Scenario: Trusted agent runs a shell command without a prompt
- **WHEN** a native API agent with the trust setting enabled requests a shell tool call
- **THEN** the system SHALL execute it immediately without an approval prompt

#### Scenario: Trusted agent writes a file without a prompt
- **WHEN** a native API agent with the trust setting enabled requests a file tool call with a write operation
- **THEN** the system SHALL execute it immediately without an approval prompt

#### Scenario: Trusted agent edits a file without a prompt
- **WHEN** a native API agent with the trust setting enabled requests a file-edit tool call
- **THEN** the system SHALL execute it immediately without an approval prompt

#### Scenario: Untrusted agent is unaffected
- **WHEN** a native API agent without the trust setting enabled requests a shell call, a file write, or a file edit
- **THEN** the system SHALL require approval exactly as it did before this capability existed
