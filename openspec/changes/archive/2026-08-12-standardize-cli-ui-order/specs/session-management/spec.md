## MODIFIED Requirements

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
- **AND** the built-in CLI group SHALL order Claude Code, Codex CLI, OpenCode, Antigravity CLI, then Gemini CLI and use the first selectable built-in CLI as the default
- **AND** grouping SHALL NOT change the stable id submitted for the selected Agent

