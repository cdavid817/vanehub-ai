# agent-chat-configuration Specification

## Purpose
TBD - created by archiving change add-agent-chat-configuration. Update Purpose after archive.
## Requirements
### Requirement: Extended thinking for Anthropic-interface API agents
The system SHALL enable extended thinking on a generation request when the session's chat configuration has thinking enabled and the agent's interface format is Anthropic.

#### Scenario: Thinking enabled and interface format is Anthropic
- **WHEN** a user starts a generation with thinking enabled for a native API agent whose interface format is Anthropic
- **THEN** the request sent to the provider SHALL enable extended thinking
- **AND** any thinking content the provider streams back SHALL be delivered to the chat UI the same way it already is for any other thinking content

#### Scenario: Thinking enabled but interface format is not Anthropic
- **WHEN** a user starts a generation with thinking enabled for a native API agent whose interface format is OpenAI-compatible
- **THEN** the request sent to the provider SHALL NOT be modified to request thinking

#### Scenario: Thinking disabled
- **WHEN** a user starts a generation with thinking disabled
- **THEN** the request sent to the provider SHALL NOT enable extended thinking, regardless of interface format

### Requirement: Reasoning depth for OpenAI-compatible-interface API agents
The system SHALL pass the session's configured reasoning depth to the provider when the agent's interface format is OpenAI-compatible.

#### Scenario: Reasoning depth set and interface format is OpenAI-compatible
- **WHEN** a user starts a generation with a reasoning depth selected for a native API agent whose interface format is OpenAI-compatible
- **THEN** the request sent to the provider SHALL include the selected reasoning effort
- **AND** the highest configurable reasoning depth SHALL map to the provider's own highest standard reasoning-effort value

#### Scenario: Reasoning depth set but interface format is Anthropic
- **WHEN** a user starts a generation with a reasoning depth selected for a native API agent whose interface format is Anthropic
- **THEN** the request sent to the provider SHALL NOT be modified because of the reasoning depth selection

#### Scenario: No reasoning depth selected
- **WHEN** a user starts a generation without selecting a reasoning depth
- **THEN** the request sent to the provider SHALL NOT include a reasoning-effort field

### Requirement: Plan mode restricts a native API agent to read-only tools
The system SHALL, when the session's permission mode is plan mode, offer a native API agent only tools that cannot modify the user's system or call an arbitrary network or tool server, while allowing configured read-only LSP queries against an explicitly trusted local workspace. It SHALL reject any attempt to use a tool or tool operation outside that restricted set regardless of what the model requests.

#### Scenario: Plan mode excludes shell and MCP-sourced tools from the catalog
- **WHEN** a generation starts in plan mode
- **THEN** the tool catalog offered to the model SHALL NOT include the shell tool, the file-edit tool, or any MCP-sourced tool

#### Scenario: Plan mode narrows the file tool to read-only
- **WHEN** a generation starts in plan mode
- **THEN** the tool catalog offered to the model SHALL only allow the file tool's read operation, not its write operation

#### Scenario: Plan mode retains read-only search tools
- **WHEN** a generation starts in plan mode
- **THEN** the tool catalog offered to the model SHALL include the content-search and filename-search tools

#### Scenario: Plan mode retains configured read-only LSP tools
- **WHEN** a generation starts in plan mode for a trusted local workspace with LSP available
- **THEN** the catalog SHALL include `find_definition`, `find_references`, `get_hover`, and `get_diagnostics`

#### Scenario: Plan mode still allows saving memories
- **WHEN** a generation starts in plan mode
- **THEN** the tool catalog offered to the model SHALL still include the remember tool

#### Scenario: A disallowed tool call is rejected even if requested
- **WHEN** the model requests the shell tool, the file-edit tool, an MCP-sourced tool, a file write operation, or an unadvertised mutating LSP operation while the session is in plan mode
- **THEN** the system SHALL reject the call as an error outcome without executing it, regardless of whether the tool appeared in the offered catalog

#### Scenario: Other permission modes are unaffected
- **WHEN** a generation starts with a permission mode other than plan mode
- **THEN** the tool catalog and tool execution behavior SHALL remain governed by that mode's existing permission and tool-availability rules

### Requirement: Context compaction is unaffected by turn-level generation settings
The system SHALL NOT apply a session's thinking, reasoning-depth, or permission-mode settings to context compaction's own internal summarization request.

#### Scenario: Compaction runs with thinking enabled for the turn
- **WHEN** context compaction triggers during a generation for which thinking is enabled
- **THEN** the internal summarization request SHALL NOT enable extended thinking

#### Scenario: Compaction runs with a reasoning depth selected for the turn
- **WHEN** context compaction triggers during a generation for which a reasoning depth is selected
- **THEN** the internal summarization request SHALL NOT include a reasoning-effort field

