## ADDED Requirements

### Requirement: Correlated native MCP telemetry
VaneHub-managed MCP connection and request flows SHALL emit correlated lifecycle telemetry with method, transport, server classification, outcome, duration, and safe error classification.

#### Scenario: Connection test runs within an operation
- **WHEN** a user starts an MCP connection test
- **THEN** the MCP telemetry SHALL correlate with the existing operation id and execution context when present
- **AND** the existing observable operation status and final test result SHALL remain available

#### Scenario: MCP request payload contains sensitive content
- **WHEN** an MCP request or response contains headers, credentials, resource content, tool arguments, or tool results
- **THEN** metadata-only telemetry SHALL omit the payload content before local persistence, logging, or export

### Requirement: Opt-in managed MCP relay
The desktop runtime SHALL provide high-fidelity Agent-to-MCP observation only through an explicitly enabled, invocation-scoped relay for VaneHub-managed MCP configurations supported by the selected Agent provider adapter.

#### Scenario: Supported managed relay is enabled
- **WHEN** a task uses a VaneHub-managed MCP configuration, relay observation is enabled, and the provider supports invocation-scoped configuration
- **THEN** the runtime SHALL forward the MCP protocol without mutating the user's global provider configuration
- **AND** it SHALL record correlated proxied MCP request lifecycle telemetry

#### Scenario: Relay is disabled or unsupported
- **WHEN** relay observation is disabled or the selected provider cannot accept invocation-scoped MCP configuration
- **THEN** Agent execution SHALL continue through its existing supported path
- **AND** MCP visibility SHALL be reported as inferred or opaque rather than proxied

### Requirement: MCP relay protocol compatibility
The managed relay SHALL preserve supported MCP stdio and HTTP protocol behavior, cancellation, timeout, session, and error semantics while adding observability.

#### Scenario: Relay forwards stdio request
- **WHEN** an Agent sends a supported MCP request through a managed stdio relay
- **THEN** the relay SHALL forward the JSON-RPC request and response without interpreting payload content as shell commands
- **AND** it SHALL retain invocation-scoped correlation for the resulting span

#### Scenario: Relay forwarding fails
- **WHEN** the upstream MCP server times out, disconnects, or returns a protocol error
- **THEN** the relay SHALL return a protocol-compatible failure to the Agent
- **AND** it SHALL record a bounded error classification without leaking payload content or credentials

### Requirement: MCP observation capability reporting
The system SHALL expose whether each Agent and MCP transport combination supports native, proxied, inferred, or opaque observation.

#### Scenario: Provider capability is queried
- **WHEN** the settings or execution timeline requests MCP observation capability
- **THEN** the service SHALL return the verified capability for the stable Agent id and transport
- **AND** availability checking SHALL NOT launch an interactive Agent session

