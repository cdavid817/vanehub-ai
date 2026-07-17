## MODIFIED Requirements

### Requirement: Persisted CLI detection status
The system SHALL store the last known CLI detection status and bounded installation distribution for supported CLI tools.

#### Scenario: Read last known detection status
- **WHEN** the frontend requests CLI tool status for initial rendering
- **THEN** the system SHALL return persisted installed state, current version, latest version, available versions, active detected path, discovered installations, conflict state, last checked time, and last error when available

#### Scenario: Represent never detected CLI
- **WHEN** a supported CLI has no persisted detection result
- **THEN** the system SHALL return a status that distinguishes never detected from installed, broken, conflicting, unsupported, and not installed states

#### Scenario: Read an older cached row
- **WHEN** a persisted CLI row predates installation-distribution fields
- **THEN** the system SHALL preserve its existing summary values and return empty or unknown additive detail fields without failing the cached read

## ADDED Requirements

### Requirement: CLI installation distribution
The backend-owned CLI catalog SHALL report a bounded normalized distribution of discovered local installations for each supported stable agent id.

#### Scenario: Report discovered installations
- **WHEN** detection finds one or more executable candidates for a supported CLI
- **THEN** each installation SHALL include its path, detected version when runnable, runnable state, user-displayable failure when broken, descriptive source, environment type, and whether it is the active PATH entry

#### Scenario: Deduplicate executable targets
- **WHEN** multiple shims or candidate paths resolve to the same executable target
- **THEN** the system SHALL avoid reporting duplicate installation records for the same normalized target

#### Scenario: Report installation conflict
- **WHEN** multiple distinct installations exist and their versions or runnable states differ
- **THEN** the CLI status SHALL identify a conflict without hiding the active PATH entry

### Requirement: CLI lifecycle eligibility metadata
The backend SHALL derive lifecycle eligibility from stable agent metadata and the detected active installation rather than display text or frontend command strings.

#### Scenario: Active npm installation
- **WHEN** the active installation is positively classified as npm-managed
- **THEN** the system SHALL report that the existing versioned npm lifecycle operation is eligible

#### Scenario: Active non-npm or unknown installation
- **WHEN** the active installation is classified as non-npm or unknown
- **THEN** the system SHALL report a manual or source-native guidance state and SHALL NOT imply that npm will update the active executable

