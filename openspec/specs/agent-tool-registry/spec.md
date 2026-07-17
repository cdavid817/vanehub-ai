# agent-tool-registry Specification

## Purpose
TBD - created by archiving change unify-ai-agent-tool-management. Update Purpose after archive.
## Requirements
### Requirement: Registered agent catalog
The system SHALL maintain a catalog of supported AI coding agents with stable identifiers, display names, provider names, launch metadata, supported interaction modes, availability state, and capability tags.

#### Scenario: Display registered agents
- **WHEN** a user opens the agent selection surface
- **THEN** the system lists each registered agent with its name, provider, availability state, and supported interaction modes

#### Scenario: Preserve stable agent identifiers
- **WHEN** an agent is displayed, selected, or referenced by saved configuration
- **THEN** the system uses the agent's stable identifier instead of relying on display text

### Requirement: Agent availability status
The system SHALL report whether each registered agent is available before the user starts a workflow.

#### Scenario: Agent is available
- **WHEN** a registered agent passes its availability check
- **THEN** the system marks the agent as selectable

#### Scenario: Agent is unavailable
- **WHEN** a registered agent fails its availability check
- **THEN** the system marks the agent as unavailable and provides a reason suitable for user display

### Requirement: Agent capability metadata
The system SHALL associate each registered agent with capability metadata that can be used for filtering, comparison, and future routing decisions.

#### Scenario: Filter by capability
- **WHEN** a user or workflow requests agents with a specific capability tag
- **THEN** the system returns only registered agents that declare that capability tag

### Requirement: SDK-backed agent readiness
The system SHALL be able to use managed SDK dependency status as a readiness signal for agents whose workflows require a managed SDK.

#### Scenario: SDK-backed agent dependency installed
- **WHEN** an agent declares a dependency on a managed SDK and that SDK is installed
- **THEN** the system SHALL allow the agent availability check to treat the managed SDK dependency as satisfied

#### Scenario: SDK-backed agent dependency missing
- **WHEN** an agent declares a dependency on a managed SDK and that SDK is not installed
- **THEN** the system SHALL mark the agent as unavailable or partially unavailable with a reason that identifies the missing SDK dependency

#### Scenario: SDK readiness check does not launch
- **WHEN** the system checks whether a managed SDK dependency is installed for agent availability
- **THEN** the system SHALL NOT launch an interactive agent session

### Requirement: Generated agent registry contracts
Agent registry entry models used by the Rust/Tauri layer and frontend service layer SHALL participate in the shared contract generation or verification workflow.

#### Scenario: Agent model changes
- **WHEN** the backend agent registry entry shape changes
- **THEN** the matching TypeScript model used by frontend services SHALL be updated or verified by the contract workflow

#### Scenario: Stable ids preserved in contracts
- **WHEN** agent registry contracts are generated or verified
- **THEN** the contract SHALL preserve stable kebab-case agent ids as the canonical reference field

### Requirement: Supported CLI tool management catalog
The system SHALL maintain backend-owned management metadata for the supported AI coding CLI tools using stable agent identifiers.

#### Scenario: List managed CLI tools
- **WHEN** CLI management status is requested
- **THEN** the system SHALL return Claude Code, Codex CLI, Gemini CLI, and OpenCode in the fixed management order using their stable agent ids

#### Scenario: Preserve CLI package metadata
- **WHEN** the system manages a supported CLI tool
- **THEN** it SHALL associate the stable agent id with its executable name and npm package name from backend-owned metadata

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

### Requirement: CLI version metadata
The system SHALL track current local version, latest remote npm version, and selectable stable npm versions for each supported CLI.

#### Scenario: Store stable available versions
- **WHEN** remote npm versions are refreshed for a CLI
- **THEN** the system SHALL store at most the latest 20 stable versions by default and exclude prerelease versions

#### Scenario: Store detected executable path
- **WHEN** a CLI executable is found locally
- **THEN** the system SHALL store the resolved local executable path for display

### Requirement: Web runtime CLI detection honesty
The Web runtime SHALL NOT fake local CLI installation status.

#### Scenario: Web runtime cannot inspect native CLI tools
- **WHEN** the CLI management page is rendered outside the Tauri desktop runtime without a native detection backend
- **THEN** the Web adapter SHALL return unsupported or undetected CLI status rather than reporting mock installed tools

### Requirement: CLI unified operation log persistence
CLI detection and package operations SHALL persist operation logs through unified log management.

#### Scenario: Persist CLI detection logs
- **WHEN** a CLI detection or remote version refresh operation emits diagnostic or operation output
- **THEN** the system SHALL write the redacted output to the active log directory with agent id and operation context

#### Scenario: Persist CLI package logs
- **WHEN** a CLI install, upgrade, or downgrade operation emits stdout, stderr, completion, or failure output
- **THEN** the system SHALL write the redacted output to the active log directory with agent id and operation context

#### Scenario: Keep CLI card logs
- **WHEN** CLI operation logs are persisted through unified log management
- **THEN** the CLI management page SHALL still display the latest operation logs inside the affected CLI card
