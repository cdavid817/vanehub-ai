## MODIFIED Requirements

### Requirement: MCP server configuration model
The system SHALL represent MCP server configurations with a globally unique kebab-case name; an explicit `stdio`, legacy `sse`, or `streamable_http` transport type; transport-specific fields; description; active flag; scope; and project path metadata.

#### Scenario: Create valid stdio server
- **WHEN** a user creates an MCP server with a kebab-case name, `stdio` transport type, and non-empty command
- **THEN** the system SHALL persist the server configuration

#### Scenario: Create valid legacy SSE server
- **WHEN** a user creates an MCP server with a kebab-case name, `sse` transport type, and non-empty URL
- **THEN** the system SHALL persist the server as a legacy MCP SSE configuration

#### Scenario: Create valid Streamable HTTP server
- **WHEN** a user creates an MCP server with a kebab-case name, `streamable_http` transport type, and non-empty URL
- **THEN** the system SHALL persist the server as an MCP Streamable HTTP configuration

#### Scenario: Migrate historical SSE rows
- **WHEN** the desktop runtime opens a database created before truthful URL transport semantics were introduced
- **THEN** it SHALL transactionally migrate historical `sse` rows to `streamable_http` so their previously effective protocol behavior is preserved
- **AND** it SHALL journal only the rows changed by that migration for a controlled rollback

#### Scenario: Reject unknown transport
- **WHEN** a configuration, import entry, or persisted row contains an unrecognized transport value
- **THEN** the system MUST reject it with a validation or migration error
- **AND** it MUST NOT silently reinterpret the value as `stdio`

#### Scenario: Reject invalid name
- **WHEN** a user creates or renames an MCP server with an empty name, uppercase letters, spaces, underscores, or leading or trailing hyphens
- **THEN** the system MUST reject the configuration with a validation error

#### Scenario: Reject duplicate name
- **WHEN** a user creates, imports, or renames an MCP server to a name already used by any MCP server in any scope
- **THEN** the system MUST reject or skip that server name rather than overwriting the existing configuration

### Requirement: MCP service adapter boundary
The frontend SHALL expose MCP operations, transport validation, limits validation, and safe failure codes through a TypeScript service interface with runtime-specific adapters.

#### Scenario: Desktop adapter uses Tauri commands
- **WHEN** the frontend runs in the Tauri desktop runtime and an MCP operation is requested
- **THEN** the MCP Tauri adapter SHALL call the matching Rust command through `invoke()`

#### Scenario: React components avoid direct invoke
- **WHEN** MCP React components load, mutate, import, export, or test server configurations
- **THEN** they SHALL call the MCP service interface and SHALL NOT import or call Tauri `invoke()` directly

#### Scenario: Web runtime uses mock adapter
- **WHEN** the frontend runs outside the Tauri desktop runtime
- **THEN** the MCP service factory SHALL use a Web adapter that returns mock data without requiring native commands

#### Scenario: Runtime adapters validate the same contract
- **WHEN** either runtime receives an unknown transport or an MCP value that exceeds a shared frontend-contract limit
- **THEN** its adapter SHALL reject the value with the same safe failure code
- **AND** the Web adapter SHALL NOT simulate native process, network, SQLite, filesystem, or logging side effects

### Requirement: MCP connection testing
The system SHALL support explicit real MCP test connections for `stdio`, legacy `sse`, and `streamable_http` transports using owned one-shot client sessions whose work, cancellation, and cleanup share one absolute operation deadline.

#### Scenario: Test stdio server
- **WHEN** a user tests a valid `stdio` MCP server
- **THEN** the desktop backend SHALL start a contained one-shot MCP child process using the configured command, args, and env, initialize the server, list available tools, and return the result

#### Scenario: Test legacy SSE server
- **WHEN** a user tests a valid `sse` MCP server
- **THEN** the desktop backend SHALL open the legacy SSE event stream, use its negotiated message endpoint, initialize the server, list available tools, and return the result

#### Scenario: Test Streamable HTTP server
- **WHEN** a user tests a valid `streamable_http` MCP server
- **THEN** the desktop backend SHALL use MCP Streamable HTTP with the configured URL and headers, initialize the server, list available tools, and return the result

#### Scenario: Test inactive server
- **WHEN** a user manually tests an inactive MCP server
- **THEN** the system SHALL perform the test connection even though the server is disabled for normal use

#### Scenario: Connection timeout releases resources
- **WHEN** an MCP test connection cannot initialize and list tools within its operation deadline
- **THEN** the system SHALL cancel pending protocol and I/O work, close the HTTP session or terminate and reap the owned child process tree, and return a failed test result with safe code `timeout`
- **AND** the operation MUST NOT become terminal while owned MCP work remains live

#### Scenario: Connection test is cancelled
- **WHEN** cancellation is requested while an MCP test is running
- **THEN** the system SHALL stop protocol work, release all owned session resources within the operation deadline, and return safe code `cancelled`

#### Scenario: Successful test performs bounded shutdown
- **WHEN** an MCP test has initialized and listed a valid bounded catalog
- **THEN** the system SHALL close its one-shot session and release owned resources before reporting the operation as succeeded

### Requirement: Claude Desktop MCP import
The system SHALL import bounded MCP server definitions from Claude Desktop compatible JSON objects with a top-level `mcpServers` object and preserve explicit URL transport semantics.

#### Scenario: Import stdio server
- **WHEN** an import entry contains a `command` field
- **THEN** the system SHALL import it as a `stdio` MCP server using its command, args, and env fields

#### Scenario: Import explicit legacy SSE server
- **WHEN** a URL import entry declares compatible type `sse`
- **THEN** the system SHALL import it as a legacy `sse` server using its URL and headers fields

#### Scenario: Import explicit Streamable HTTP server
- **WHEN** a URL import entry declares compatible type `http` or `streamable_http`
- **THEN** the system SHALL import it as a `streamable_http` server using its URL and headers fields

#### Scenario: Import URL server without type
- **WHEN** an import entry contains a `url` field, no `command` field, and no recognized type marker
- **THEN** the system SHALL import it as `streamable_http` to preserve VaneHub's historical effective URL behavior

#### Scenario: Skip import conflict
- **WHEN** an import entry name conflicts with an existing MCP server name
- **THEN** the system SHALL skip that entry and include its name in the skipped result list

#### Scenario: Reject oversized import before parsing
- **WHEN** an import document exceeds 1 MiB or contains more than 128 server entries
- **THEN** both the frontend service flow and native boundary MUST reject it with safe code `limit_exceeded`
- **AND** the frontend SHALL enforce the byte limit before parsing the JSON text

### Requirement: Claude Desktop MCP export
The system SHALL export selected MCP servers as Claude Desktop compatible JSON with explicit transport semantics and without VaneHub internal metadata.

#### Scenario: Export selected servers
- **WHEN** a user selects MCP servers for export
- **THEN** the system SHALL produce a JSON object with a top-level `mcpServers` object containing only those selected server names

#### Scenario: Exclude internal fields
- **WHEN** the system exports MCP servers
- **THEN** the exported entries MUST exclude scope, project path, active state, description, cached status, timestamps, migration metadata, and other VaneHub-only metadata

#### Scenario: Export stdio transport fields
- **WHEN** the system exports a `stdio` server
- **THEN** the exported entry SHALL include command, args, and env fields relevant to that transport

#### Scenario: Export legacy SSE transport fields
- **WHEN** the system exports an `sse` server
- **THEN** the exported entry SHALL include its URL, headers, and compatible `sse` type marker

#### Scenario: Export Streamable HTTP transport fields
- **WHEN** the system exports a `streamable_http` server
- **THEN** the exported entry SHALL include its URL, headers, and compatible `http` type marker

### Requirement: MCP P1 deferred behavior
The system SHALL keep the settings-page tool invocation workflow deferred while treating all transport options exposed by that page as supported rather than reserved placeholders.

#### Scenario: Tool calling UI deferred
- **WHEN** the MCP settings page displays discovered MCP tools
- **THEN** the page SHALL NOT expose a settings-page workflow for invoking those tools

#### Scenario: Streamable HTTP is no longer reserved
- **WHEN** a user configures and tests a valid `streamable_http` MCP server
- **THEN** the desktop runtime SHALL execute a real Streamable HTTP connection instead of returning a reserved-transport error

### Requirement: Observable MCP connection tests
MCP connection tests SHALL expose observable operation state and safe structured command audit while preventing operation completion before terminal resource cleanup.

#### Scenario: MCP test operation starts
- **WHEN** a user starts a connection test for an MCP server
- **THEN** the system SHALL expose operation status or progress through the MCP service boundary while preserving the existing final test result behavior

#### Scenario: MCP test command audit
- **WHEN** a stdio MCP test starts a configured external command
- **THEN** the native runtime SHALL record a command execution audit entry associated with the test operation
- **AND** the entry MUST omit environment values and raw arguments while retaining safe executable classification, argument count, transport, server identity, correlation, and outcome metadata

#### Scenario: MCP test reaches a terminal operation status
- **WHEN** an MCP connection test succeeds, fails, is cancelled, or times out
- **THEN** its operation SHALL reach the matching terminal status only after owned session cleanup finishes
- **AND** a cleanup failure SHALL produce a failed operation with safe code `cleanup`

### Requirement: MCP relay protocol compatibility
The managed relay SHALL preserve stdio, legacy SSE, and Streamable HTTP JSON-RPC behavior, bidirectional messages, cancellation, per-request deadlines, session lifecycle, and protocol errors while adding observability.

#### Scenario: Relay forwards bidirectional stdio traffic
- **WHEN** an Agent or stdio MCP server sends a bounded JSON-RPC request, response, or notification through the managed relay
- **THEN** the relay SHALL forward the message without interpreting payload content as shell commands
- **AND** it SHALL correlate a request span with its response rather than completing the span when request bytes are written

#### Scenario: Stdio request times out while parent input remains open
- **WHEN** a forwarded stdio request exceeds its deadline and the Agent keeps relay stdin open
- **THEN** the relay SHALL return a protocol-compatible timeout for that request, terminate and reap the upstream process tree, and exit without waiting indefinitely for parent EOF

#### Scenario: Upstream stdio child exits while parent input remains open
- **WHEN** the upstream stdio child exits or disconnects while the Agent keeps relay stdin open
- **THEN** the relay SHALL finalize bounded telemetry and exit without joining a permanently blocked parent-input reader

#### Scenario: Streamable HTTP returns JSON
- **WHEN** a Streamable HTTP request returns a successful `application/json` JSON-RPC message within limits
- **THEN** the relay SHALL emit exactly that message using newline-delimited stdio framing

#### Scenario: Streamable HTTP returns SSE
- **WHEN** a Streamable HTTP request returns a successful `text/event-stream` response
- **THEN** the relay SHALL incrementally parse bounded SSE events and emit each JSON `data` message using newline-delimited stdio framing
- **AND** it SHALL NOT forward raw SSE control lines to the Agent

#### Scenario: Streamable HTTP acknowledges a notification
- **WHEN** a Streamable HTTP notification receives `202 Accepted` without a JSON-RPC response
- **THEN** the relay SHALL treat it as acknowledged and SHALL NOT emit an empty response frame

#### Scenario: Legacy SSE relay negotiates its message endpoint
- **WHEN** an Agent connects through a managed legacy `sse` relay
- **THEN** the relay SHALL maintain the server event stream, send requests to its negotiated message endpoint, and forward bounded JSON-RPC event data to the Agent

#### Scenario: Relay closes a Streamable HTTP session
- **WHEN** a managed Streamable HTTP relay with an established `Mcp-Session-Id` shuts down normally or by cancellation
- **THEN** it SHALL attempt a bounded session `DELETE` and release the HTTP stream before terminal completion

#### Scenario: Relay forwarding fails
- **WHEN** the upstream MCP server times out, disconnects, redirects, returns a non-success status, produces invalid framing, or exceeds a resource limit
- **THEN** the relay SHALL return a protocol-compatible failure associated with the originating request id when available
- **AND** it SHALL record a bounded safe error classification without leaking payload content or credentials

## ADDED Requirements

### Requirement: MCP runtime resource limits
The system MUST enforce central MCP resource limits at the earliest controllable ingress in both frontend contract validation and the native runtime, with backend validation remaining authoritative.

#### Scenario: Reject oversized server configuration collections
- **WHEN** one server configuration contains more than 128 args, 128 environment entries, or 128 headers, or its serialized transport configuration exceeds 256 KiB
- **THEN** the system MUST reject it with safe code `limit_exceeded` before persisting or launching the server

#### Scenario: Reject oversized protocol input
- **WHEN** a JSON-RPC frame, SSE event, or HTTP response body exceeds 2 MiB
- **THEN** the receiving transport MUST stop reading at limit plus one, fail with safe code `limit_exceeded`, and perform bounded session cleanup

#### Scenario: Reject oversized discovered catalog
- **WHEN** a server exposes more than 128 tools, more than 2 MiB of serialized catalog data, a tool name longer than 256 UTF-8 bytes, a description longer than 8 KiB, or one input schema larger than 128 KiB or deeper than 32 JSON levels
- **THEN** the connection test MUST fail with safe code `limit_exceeded`
- **AND** the oversized payload MUST NOT replace that server's cached tool catalog

#### Scenario: Read corrupt or oversized cached catalog
- **WHEN** one server's legacy cached catalog is malformed or exceeds current MCP limits
- **THEN** the system SHALL exclude that server's tools and emit one bounded safe diagnostic
- **AND** it SHALL continue processing valid servers rather than failing the entire visible catalog

### Requirement: Protected transient MCP relay configuration
The desktop runtime MUST store invocation-scoped relay configuration in a private, uniquely named VaneHub-owned directory and MUST clean every owned artifact without changing the existing plaintext SQLite or export contract.

#### Scenario: Create relay artifacts
- **WHEN** the runtime prepares relay files containing MCP environment values, headers, database location, or execution context
- **THEN** it MUST create a unique per-invocation directory with current-user-only access before writing secret-bearing bytes
- **AND** it MUST use exclusive file creation with unpredictable names

#### Scenario: Relay preparation partially fails
- **WHEN** one server or provider configuration fails after earlier relay artifacts were created
- **THEN** the runtime MUST remove every artifact already owned by that preparation before returning the failure

#### Scenario: Relay invocation terminates
- **WHEN** an Agent invocation completes, fails, is cancelled, or times out
- **THEN** the owning relay guard MUST idempotently remove its provider and per-server configuration artifacts after verifying their canonical paths remain inside the dedicated relay root

#### Scenario: Relay helper consumes its configuration
- **WHEN** a relay helper successfully opens its configuration file
- **THEN** it SHALL unlink that file before connecting to the upstream MCP server

#### Scenario: Recover stale relay artifacts
- **WHEN** desktop startup finds a versioned VaneHub-owned relay invocation directory older than 24 hours
- **THEN** the runtime SHALL remove it only after canonical-root verification and SHALL log metadata-only cleanup counts
- **AND** it MUST NOT delete unrelated system temporary files

### Requirement: Contained MCP child diagnostics
The desktop runtime SHALL drain MCP child stderr concurrently through a bounded path and SHALL route any persisted diagnostic through unified logging and redaction rather than inheriting native stderr.

#### Scenario: MCP child writes stderr
- **WHEN** a connection test or managed relay child emits stderr
- **THEN** the runtime SHALL drain it without allowing the child pipe to block and SHALL retain at most 64 KiB for diagnostic summarization
- **AND** it MUST NOT copy the raw bytes directly to native stderr

#### Scenario: Persist MCP child failure diagnostic
- **WHEN** an MCP child spawn, timeout, cancellation, protocol, non-zero exit, or cleanup failure requires a persistent diagnostic
- **THEN** the runtime SHALL write only a bounded redacted summary with safe operation, server, transport, outcome, truncation, and correlation metadata through unified logging
- **AND** it MUST omit raw arguments, environment values, headers, protocol bodies, schemas, tool arguments, and tool results

#### Scenario: Unified logging is unavailable
- **WHEN** the normal unified logging sink cannot accept an MCP child diagnostic
- **THEN** any emergency sink SHALL receive only an already-redacted fixed failure classification
- **AND** it MUST NOT receive captured stderr or serialized relay configuration

### Requirement: Safe MCP runtime failure contract
The system SHALL classify MCP runtime failures as `validation`, `spawn`, `timeout`, `cancelled`, `protocol`, `upstream_http`, `limit_exceeded`, `transport`, or `cleanup` and SHALL expose only concise safe messages and additive safe codes across service boundaries.

#### Scenario: Native MCP operation fails
- **WHEN** a connection, relay, migration, validation, limit, or cleanup operation fails
- **THEN** the native runtime SHALL map the failure to one safe code and concise user-displayable text
- **AND** telemetry SHALL record the code without deriving it by parsing free-form upstream messages

#### Scenario: Web MCP validation fails
- **WHEN** the Web/mock adapter receives an equivalent invalid transport or oversized contract value
- **THEN** it SHALL return the same safe code and SHALL NOT claim that a native MCP process or network attempt occurred

