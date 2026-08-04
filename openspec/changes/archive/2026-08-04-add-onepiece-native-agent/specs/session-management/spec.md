## ADDED Requirements

### Requirement: Service-backed session Agent discovery
The create-session UI SHALL derive candidate Agents from service-backed registry entries and their declared interaction modes and availability rather than from a fixed stable-id allowlist.

#### Scenario: List CLI and API session candidates
- **WHEN** the registry contains Agents that declare `cli` or `api` interaction support
- **THEN** the create-session UI SHALL present those Agents as Single-Agent candidates
- **AND** it SHALL NOT require their ids to appear in a frontend eligibility allowlist

#### Scenario: Exclude non-chat-only candidates
- **WHEN** a registry entry declares only `browser` or `native-desktop` interaction support
- **THEN** the create-session UI SHALL NOT present that entry as a chat-session candidate

#### Scenario: Show unconfigured OnePiece
- **WHEN** OnePiece is present but non-selectable because its provider configuration or credential is incomplete
- **THEN** the create-session UI SHALL show a disabled OnePiece candidate with its readiness reason
- **AND** it SHALL provide an action to open OnePiece configuration

#### Scenario: Group candidate presentation
- **WHEN** the UI renders built-in OnePiece, built-in CLI Agents, and user-created API Agents
- **THEN** it SHALL present OnePiece in the VaneHub-native group, CLI Agents in the built-in CLI group, and user API Agents in a custom API group
- **AND** it SHALL order those groups as built-in CLI, VaneHub native, then custom API
- **AND** the built-in CLI group SHALL order Codex CLI, Claude Code, Gemini CLI, then OpenCode and use the first selectable built-in CLI as the default
- **AND** grouping SHALL NOT change the stable id submitted for the selected Agent

## MODIFIED Requirements

### Requirement: Session creation input
The system SHALL create sessions from a service-level input that includes stable agent id, interaction mode, selected project path, and optional worktree request. The native and Web boundaries SHALL accept a declared `api` mode as well as existing supported modes and SHALL validate the selected Agent's identity, declared mode, and readiness before persisting the session.

#### Scenario: Create session for selected agent
- **WHEN** the user creates a session for Claude Code, Gemini CLI, Codex CLI, or OpenCode using a declared CLI mode
- **THEN** the created session SHALL store the selected stable agent id rather than matching by display name

#### Scenario: Create session for OnePiece
- **WHEN** the user selects a ready OnePiece and submits a local Single-Agent session
- **THEN** the frontend SHALL submit `agentId = onepiece` and `interactionMode = api`
- **AND** the created session SHALL persist those stable values

#### Scenario: Reject unsupported agent
- **WHEN** session creation receives an unknown Agent id or a mode that the selected Agent does not declare
- **THEN** the system SHALL reject the request without creating a session

#### Scenario: Reject a non-ready API Agent
- **WHEN** session creation receives an API Agent whose availability is not selectable
- **THEN** the native or Web boundary SHALL reject the request with a safe readiness reason
- **AND** it SHALL NOT contact the provider or create a session

#### Scenario: Create session uses selected folder
- **WHEN** the user creates a session without worktree creation
- **THEN** the created session SHALL use the selected project folder as the effective folder

#### Scenario: Reject remote OnePiece session
- **WHEN** session creation combines `agentId = onepiece` with a remote workspace request
- **THEN** the frontend SHALL prevent submission or the service SHALL reject it
- **AND** the system SHALL explain that first-version OnePiece sessions require a local project or local worktree

#### Scenario: Preserve Web runtime behavior
- **WHEN** the app runs in Web mode
- **THEN** the Web adapter SHALL accept the same supported session creation input and return equivalent mock session metadata
- **AND** it SHALL enforce equivalent Agent mode, readiness, and workspace restrictions

### Requirement: Derived session visual identity
The system SHALL derive session icon identity from the session's stable agent id rather than persisting redundant icon metadata in the session entity, including for OnePiece, built-in CLI Agents, and user-created API Agents.

#### Scenario: Store stable agent id only
- **WHEN** a session is created for OnePiece, Claude Code, Gemini CLI, Codex CLI, OpenCode, or another eligible API Agent
- **THEN** the session record SHALL store the selected stable agent id
- **AND** it SHALL NOT require a persisted icon name, icon path, or icon color field

#### Scenario: Derive icon after reload
- **WHEN** persisted sessions are listed after app restart or Web/mock reload
- **THEN** the UI SHALL render a known first-party identity from the stable Agent id
- **AND** it SHALL render the generic Agent identity for an unrecognized user-created id

### Requirement: Single-Agent session mode
The system SHALL create first-version interactive CLI or API chat sessions as Single Agent sessions owned by the stable agent id selected in the create-session dialog.

#### Scenario: Create Single Agent session
- **WHEN** the user submits the create-session dialog in Single Agent mode for Claude Code, Gemini CLI, Codex CLI, or OpenCode
- **THEN** the created session SHALL store the selected stable agent id
- **AND** that selected agent id SHALL be the Agent used for automatic Agent Terminal startup

#### Scenario: Create OnePiece Single Agent session
- **WHEN** the user submits the create-session dialog in Single Agent mode for a ready OnePiece
- **THEN** the created session SHALL store stable id `onepiece` with interaction mode `api`
- **AND** the system SHALL NOT start or offer an Agent Terminal for that session

#### Scenario: Reject Multi Agent creation
- **WHEN** session creation receives a Multi Agent first-version request
- **THEN** the system SHALL reject or prevent the request without creating a session
- **AND** it SHALL report that Multi Agent sessions are not yet implemented
