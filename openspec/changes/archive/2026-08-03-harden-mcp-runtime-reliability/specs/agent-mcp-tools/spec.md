## MODIFIED Requirements

### Requirement: MCP-sourced tools in the native tool catalog
The system SHALL include, alongside the fixed `shell`/`file`/`remember` tools, bounded catalog entries exposed by MCP servers visible and active for the current session's workspace folder, using each server's most recently cached valid tool list rather than a live connection made at catalog-build time.

#### Scenario: Visible active server contributes its cached tools
- **WHEN** a native API agent starts a generation in a session whose workspace folder matches an active MCP server's project scope (or a user-scoped active MCP server exists)
- **THEN** the tool catalog sent to the provider SHALL include entries from that server's last successful bounded "Test Connection" result

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
- **AND** it SHALL NOT fail the generation because of this failure

#### Scenario: One cached server catalog is invalid
- **WHEN** one visible server's cached tool list is corrupt, oversized, or contains an invalid tool descriptor
- **THEN** the system SHALL exclude that server, record one bounded safe diagnostic, and continue adding valid MCP servers and fixed tools

#### Scenario: Aggregate MCP catalog exceeds provider budget
- **WHEN** valid visible servers collectively expose more than 256 MCP-sourced tools for one generation
- **THEN** the system SHALL admit at most 256 MCP tools using stable server-name and tool-name ordering
- **AND** it SHALL preserve every fixed tool and record one bounded overflow warning

### Requirement: Invoking an MCP-sourced tool
The system SHALL let a native API agent invoke an approved MCP-sourced tool through an owned one-shot session, subject to bounded input, output, cancellation, deadline, and terminal cleanup semantics, and return the result as a normal tool execution outcome.

#### Scenario: Successful MCP tool call
- **WHEN** the model requests an MCP-sourced tool call, the user approves it, its arguments are valid and bounded, and the remote call succeeds with a bounded result
- **THEN** the system SHALL connect to the owning MCP server, invoke the requested tool, close the one-shot session, and return the tool's text content as the tool execution output
- **AND** it SHALL NOT report success before every owned session or process resource reaches a terminal state

#### Scenario: Reject malformed tool arguments before connecting
- **WHEN** an MCP tool call supplies arguments that are not a JSON object or null, exceed 256 KiB, or exceed 32 JSON levels
- **THEN** the system MUST return an error outcome with safe code `validation` or `limit_exceeded` before starting a process or opening a network connection

#### Scenario: MCP tool call times out
- **WHEN** MCP connection, initialization, invocation, response, or cleanup cannot finish within the absolute call deadline
- **THEN** the system SHALL cancel pending work, terminate and reap any owned process tree or close the HTTP session, and return an error outcome with safe code `timeout`
- **AND** no MCP task owned by the call may remain live after the outcome is returned

#### Scenario: Agent generation cancellation reaches MCP call
- **WHEN** the user cancels the native Agent generation while an MCP tool call is running
- **THEN** the Agent runtime SHALL propagate cancellation into the MCP session, release all owned resources within the call deadline, and return a cancelled tool outcome
- **AND** it SHALL NOT merely mark the outer Agent operation cancelled while MCP work continues

#### Scenario: MCP cleanup fails after a remote success
- **WHEN** the remote tool produces a result but the owned MCP session cannot prove terminal cleanup
- **THEN** the system MUST return an error outcome with safe code `cleanup` rather than an apparently successful tool result

#### Scenario: MCP tool call fails without failing the generation
- **WHEN** an MCP tool call cannot complete because the server connection fails, times out, is cancelled, exceeds a limit, or the remote server reports a tool-level error
- **THEN** the system SHALL return a tool execution outcome marked as an error, containing a concise safe description of the failure
- **AND** the generation SHALL continue when it has not itself been cancelled, letting the model see and react to the failure

#### Scenario: Non-text tool result content
- **WHEN** an MCP tool's result contains non-text content (image, audio, or resource blocks)
- **THEN** the system SHALL represent that content in the tool execution output with a clearly labeled placeholder rather than omitting the result entirely or failing the call

#### Scenario: Tool result exceeds the output budget
- **WHEN** rendered MCP tool output would exceed 1 MiB
- **THEN** the system MUST return an error outcome with safe code `limit_exceeded`
- **AND** it MUST NOT silently truncate the result into an apparently successful outcome or persist the oversized result in telemetry

### Requirement: Web runtime MCP tool simulation parity
The Web/mock runtime SHALL simulate bounded MCP-sourced catalog and tool-call behavior through the same service and event contracts used by desktop runtime, without a real MCP server connection.

#### Scenario: Mock MCP tool call
- **WHEN** a user exercises a native API agent's tool-use loop in Web/mock mode
- **THEN** the Web adapter SHALL simulate at least one MCP-sourced tool call and result through the same event sequence contract the desktop runtime produces for a real MCP tool call

#### Scenario: Mock MCP input exceeds a shared limit
- **WHEN** a simulated MCP catalog, argument object, or result exceeds the shared contract limit
- **THEN** the Web adapter SHALL return the same safe `limit_exceeded` code used by desktop runtime
- **AND** it SHALL NOT emit events claiming that a native process or network connection was started

