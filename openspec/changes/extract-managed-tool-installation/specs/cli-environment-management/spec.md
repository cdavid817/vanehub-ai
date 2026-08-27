## MODIFIED Requirements

### Requirement: Platform-safe vendor installer execution

The system SHALL execute vendor installers only from audited platform-specific templates, and SHALL obtain the installer file through `managed-tool-installation` rather than through a download path of its own.

#### Scenario: Windows has only a Bash template

- **WHEN** a vendor source has no approved Windows-native execution template
- **THEN** the source SHALL be unavailable for automatic Windows lifecycle actions
- **AND** the backend SHALL NOT fall through to a Unix shell template

#### Scenario: Vendor script is downloaded

- **WHEN** an approved vendor installer is executed
- **THEN** the backend SHALL obtain it through the shared managed-tool retrieval, which applies HTTPS allowlisting per redirect hop, the declared byte ceiling and deadline, cancellation, digest verification before execution, and owned temporary storage released on every exit
- **AND** it SHALL NOT use pipe-to-shell, `Invoke-Expression`, or `irm | iex`

#### Scenario: The shared retrieval refuses the artifact

- **WHEN** shared retrieval refuses an installer on the allowlist, the ceiling, the deadline, or a digest mismatch
- **THEN** the CLI lifecycle action SHALL fail with that outcome rather than proceeding
- **AND** the CLI context SHALL NOT retry the download by another route
