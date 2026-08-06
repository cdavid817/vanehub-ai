## MODIFIED Requirements

### Requirement: Plan mode restricts a native API agent to read-only tools
The system SHALL, when the session's permission mode is plan mode, offer a native API agent only tools that cannot modify the user's system or call an arbitrary external server, and SHALL reject any attempt to use a tool or tool operation outside that restricted set regardless of what the model requests.

#### Scenario: Plan mode excludes shell and MCP-sourced tools from the catalog
- **WHEN** a generation starts in plan mode
- **THEN** the tool catalog offered to the model SHALL NOT include the shell tool, the file-edit tool, or any MCP-sourced tool

#### Scenario: Plan mode narrows the file tool to read-only
- **WHEN** a generation starts in plan mode
- **THEN** the tool catalog offered to the model SHALL only allow the file tool's read operation, not its write operation

#### Scenario: Plan mode retains read-only search tools
- **WHEN** a generation starts in plan mode
- **THEN** the tool catalog offered to the model SHALL include the content-search and filename-search tools

#### Scenario: Plan mode still allows saving memories
- **WHEN** a generation starts in plan mode
- **THEN** the tool catalog offered to the model SHALL still include the remember tool

#### Scenario: A disallowed tool call is rejected even if requested
- **WHEN** the model requests the shell tool, the file-edit tool, an MCP-sourced tool, or a file write operation while the session is in plan mode
- **THEN** the system SHALL reject the call as an error outcome without executing it, regardless of whether the tool appeared in the offered catalog

#### Scenario: Other permission modes are unaffected
- **WHEN** a generation starts with a permission mode other than plan mode
- **THEN** the tool catalog and tool execution behavior SHALL be exactly what they were before this capability existed
