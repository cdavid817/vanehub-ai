## ADDED Requirements

### Requirement: MCP server configuration model
The system SHALL represent MCP server configurations with a globally unique kebab-case name, transport type, transport-specific fields, description, active flag, scope, and project path metadata.

#### Scenario: Create valid stdio server
- **WHEN** a user creates an MCP server with a kebab-case name, `stdio` transport type, and non-empty command
- **THEN** the system SHALL persist the server configuration

#### Scenario: Create valid SSE server
- **WHEN** a user creates an MCP server with a kebab-case name, `sse` transport type, and non-empty URL
- **THEN** the system SHALL persist the server configuration

#### Scenario: Reject invalid name
- **WHEN** a user creates or renames an MCP server with an empty name, uppercase letters, spaces, underscores, or leading or trailing hyphens
- **THEN** the system MUST reject the configuration with a validation error

#### Scenario: Reject duplicate name
- **WHEN** a user creates, imports, or renames an MCP server to a name already used by any MCP server in any scope
- **THEN** the system MUST reject or skip that server name rather than overwriting the existing configuration

### Requirement: MCP server scoped persistence
The system SHALL persist MCP server configurations in SQLite with user and project scopes, where project-scoped servers are associated with the current working directory absolute `project_path`.

#### Scenario: List visible servers
- **WHEN** the MCP settings page requests the server list
- **THEN** the system SHALL return all user-scoped servers and project-scoped servers whose `project_path` matches the current working directory absolute path

#### Scenario: Create project-scoped server
- **WHEN** a user creates an MCP server with project scope
- **THEN** the system SHALL persist the server with scope `project` and the current working directory absolute path as `project_path`

#### Scenario: Exclude other project servers
- **WHEN** a project-scoped MCP server belongs to a different `project_path`
- **THEN** the system SHALL NOT include it in the visible MCP server list for the current project

### Requirement: MCP server lifecycle management
The system SHALL allow users to add, edit, rename, remove, enable, and disable MCP server configurations through the MCP service boundary.

#### Scenario: Rename server
- **WHEN** a user updates an MCP server with a new valid and unused name
- **THEN** the system SHALL persist the new name and preserve the server configuration and cached status fields

#### Scenario: Disable server
- **WHEN** a user disables an MCP server
- **THEN** the system SHALL mark the server inactive without deleting its configuration or cached test result

#### Scenario: Remove server
- **WHEN** a user removes an MCP server
- **THEN** the system SHALL delete the server configuration from SQLite

### Requirement: MCP service adapter boundary
The frontend SHALL expose MCP operations through a TypeScript service interface with runtime-specific adapters.

#### Scenario: Desktop adapter uses Tauri commands
- **WHEN** the frontend runs in the Tauri desktop runtime and an MCP operation is requested
- **THEN** the MCP Tauri adapter SHALL call the matching Rust command through `invoke()`

#### Scenario: React components avoid direct invoke
- **WHEN** MCP React components load, mutate, import, export, or test server configurations
- **THEN** they SHALL call the MCP service interface and SHALL NOT import or call Tauri `invoke()` directly

#### Scenario: Web runtime uses mock adapter
- **WHEN** the frontend runs outside the Tauri desktop runtime
- **THEN** the MCP service factory SHALL use a Web adapter that returns mock data without requiring native commands

### Requirement: MCP connection testing
The system SHALL support explicit real MCP test connections for `stdio` and `sse` transports using oneshot client connections.

#### Scenario: Test stdio server
- **WHEN** a user tests a valid `stdio` MCP server
- **THEN** the desktop backend SHALL start a oneshot MCP client connection using the configured command, args, and env, initialize the server, list available tools, and return the result

#### Scenario: Test SSE server
- **WHEN** a user tests a valid `sse` MCP server
- **THEN** the desktop backend SHALL start a oneshot MCP client connection using the configured URL and headers, initialize the server, list available tools, and return the result

#### Scenario: Test inactive server
- **WHEN** a user manually tests an inactive MCP server
- **THEN** the system SHALL perform the test connection even though the server is disabled for normal use

#### Scenario: Connection timeout
- **WHEN** an MCP test connection does not initialize and list tools before the configured timeout
- **THEN** the system SHALL stop waiting and return a failed test result with a timeout error

### Requirement: MCP status cache
The system SHALL cache the latest MCP test status, discovered tools, error message, connection timestamp, and test duration in SQLite.

#### Scenario: Cache successful test
- **WHEN** an MCP test connection succeeds and discovers tools
- **THEN** the system SHALL store a connected status, discovered tools, connection timestamp, and duration for that server

#### Scenario: Cache failed test
- **WHEN** an MCP test connection fails
- **THEN** the system SHALL store an error status, error message, and duration for that server

#### Scenario: Read status without live connection
- **WHEN** the frontend requests an MCP server status
- **THEN** the system SHALL return status from cached SQLite fields without starting a process or opening a network connection

#### Scenario: Disabled status
- **WHEN** the frontend requests status for an inactive MCP server
- **THEN** the system SHALL report connection status `disabled` while preserving the last cached test details for display

### Requirement: Claude Desktop MCP import
The system SHALL import MCP servers from Claude Desktop compatible JSON objects with a top-level `mcpServers` object.

#### Scenario: Import stdio server
- **WHEN** an import entry contains a `command` field
- **THEN** the system SHALL import it as a `stdio` MCP server using its command, args, and env fields

#### Scenario: Import URL server
- **WHEN** an import entry contains a `url` field and no `command` field
- **THEN** the system SHALL import it as an `sse` MCP server using its URL and headers fields

#### Scenario: Skip import conflict
- **WHEN** an import entry name conflicts with an existing MCP server name
- **THEN** the system SHALL skip that entry and include its name in the skipped result list

### Requirement: Claude Desktop MCP export
The system SHALL export selected MCP servers as Claude Desktop compatible JSON without VaneHub internal metadata.

#### Scenario: Export selected servers
- **WHEN** a user selects MCP servers for export
- **THEN** the system SHALL produce a JSON object with a top-level `mcpServers` object containing only those selected server names

#### Scenario: Exclude internal fields
- **WHEN** the system exports MCP servers
- **THEN** the exported entries MUST exclude scope, project path, active state, description, cached status, timestamps, and other VaneHub-only metadata

#### Scenario: Export transport fields
- **WHEN** the system exports a `stdio` server
- **THEN** the exported entry SHALL include command, args, and env fields relevant to that transport

#### Scenario: Export URL transport fields
- **WHEN** the system exports an `sse` or reserved URL-based server
- **THEN** the exported entry SHALL include URL and headers fields relevant to that transport

### Requirement: MCP P1 deferred behavior
The system SHALL reserve interfaces and data model fields for later MCP capabilities without exposing them as completed P1 UI workflows.

#### Scenario: Tool calling UI deferred
- **WHEN** the MCP settings page displays discovered MCP tools
- **THEN** the page SHALL NOT expose a P1 UI workflow for invoking those tools

#### Scenario: Streamable HTTP not required for P1 completion
- **WHEN** a user configures a `streamable_http` MCP server in a reserved or future-facing flow
- **THEN** the system SHALL NOT be considered P1-complete based on `streamable_http` support, while `stdio` and `sse` real test connections MUST work

### Requirement: MCP plaintext secret handling
The system SHALL store and export MCP `env` and `headers` values as plaintext in P1.

#### Scenario: Persist plaintext secret fields
- **WHEN** a user saves MCP environment variables or headers
- **THEN** the system SHALL persist those values in SQLite as plaintext JSON

#### Scenario: Export plaintext secret fields
- **WHEN** a user exports MCP servers containing env or header values
- **THEN** the system SHALL include those values in the exported Claude Desktop JSON as plaintext
