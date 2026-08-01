## ADDED Requirements

### Requirement: MCP-sourced tools in the native tool catalog
The system SHALL include, alongside the fixed `shell`/`file`/`remember` tools, one catalog entry per tool exposed by each MCP server visible and active for the current session's workspace folder, using each server's most recently cached tool list rather than a live connection made at catalog-build time.

#### Scenario: Visible active server contributes its cached tools
- **WHEN** a native API agent starts a generation in a session whose workspace folder matches an active MCP server's project scope (or a user-scoped active MCP server exists)
- **THEN** the tool catalog sent to the provider SHALL include one entry per tool in that server's last successful "Test Connection" result

#### Scenario: Untested or failed server contributes no tools
- **WHEN** a visible, active MCP server has never been successfully tested, or its most recent test failed
- **THEN** the tool catalog SHALL NOT include any entry for that server

#### Scenario: Inactive or out-of-scope server contributes no tools
- **WHEN** an MCP server is disabled, or is project-scoped to a different project than the current session's workspace folder
- **THEN** the tool catalog SHALL NOT include any entry for that server

#### Scenario: MCP catalog names never collide with the fixed catalog
- **WHEN** an MCP server exposes a tool literally named `shell`, `file`, or `remember`
- **THEN** the corresponding catalog entry SHALL be distinguishable from the fixed tool of the same name and SHALL NOT replace or shadow it

#### Scenario: Catalog lookup failure degrades gracefully
- **WHEN** the system cannot determine the current session's visible MCP servers or their cached tools due to an internal error
- **THEN** the system SHALL log the failure and proceed with only the fixed `shell`/`file`/`remember` catalog for that generation
- **AND** SHALL NOT fail the generation because of this failure

### Requirement: Invoking an MCP-sourced tool
The system SHALL let a native API agent invoke an MCP-sourced tool the model selected from the catalog, by connecting to the owning MCP server, issuing a single tool call, and returning the result as a normal tool execution outcome.

#### Scenario: Successful MCP tool call
- **WHEN** the model requests an MCP-sourced tool call and the user approves it (or it is otherwise permitted to run)
- **THEN** the system SHALL connect to the owning MCP server, invoke the requested tool with the model-supplied arguments, and return the tool's text content as the tool execution output

#### Scenario: MCP tool call fails without failing the generation
- **WHEN** an MCP tool call cannot complete because the server connection fails, times out, or the remote server reports a tool-level error
- **THEN** the system SHALL return a tool execution outcome marked as an error, containing a description of the failure
- **AND** the generation SHALL continue, letting the model see and react to the failure

#### Scenario: Non-text tool result content
- **WHEN** an MCP tool's result contains non-text content (image, audio, or resource blocks)
- **THEN** the system SHALL represent that content in the tool execution output with a clearly labeled placeholder rather than omitting the result entirely or failing the call

### Requirement: MCP tool calls require explicit approval
The system SHALL classify every MCP-sourced tool call as requiring explicit user approval before it executes, with no automatic approval path.

#### Scenario: MCP tool call pauses for approval
- **WHEN** the model requests any MCP-sourced tool call, regardless of which server or tool
- **THEN** the system SHALL pause the tool-use loop and require an explicit user approve/deny decision before invoking the tool

#### Scenario: Denied MCP tool call
- **WHEN** a user denies an MCP-sourced tool call awaiting approval
- **THEN** the system SHALL NOT connect to the MCP server or invoke the tool
- **AND** SHALL report the denial to the model as the tool's result, matching how denied shell/file calls are reported

### Requirement: MCP server visibility is re-validated at call time
The system SHALL verify that the MCP server targeted by a tool call is still visible and active for the current session's workspace folder immediately before connecting to it, independent of whether that server appeared in the catalog offered earlier in the same generation.

#### Scenario: Call targets a server outside the current session's visibility
- **WHEN** a tool call names an MCP server that is not currently visible and active for the session's workspace folder (including a project-scoped server belonging to a different project)
- **THEN** the system SHALL reject the call as an error outcome without attempting a connection
- **AND** SHALL NOT execute any tool on that server

### Requirement: Web runtime MCP tool simulation parity
The Web/mock runtime SHALL simulate an MCP-sourced tool appearing in the catalog and being called, through the same service contracts the desktop runtime uses, without a real MCP server connection.

#### Scenario: Mock MCP tool call
- **WHEN** a user exercises a native API agent's tool-use loop in Web/mock mode
- **THEN** the Web adapter SHALL simulate at least one MCP-sourced tool call and result through the same event sequence contract the desktop runtime produces for a real MCP tool call
