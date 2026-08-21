## ADDED Requirements

### Requirement: Connector descriptors use one stable SPI

The system SHALL represent every unified connector with stable id, display metadata, connector type, source/provenance, supported authentication strategy, configuration schema, capabilities, lifecycle support, and driver identity. Built-in and extension drivers SHALL implement the same application-level Connector SPI without exposing transport/library types to UI or other contexts.

#### Scenario: Extension connector is discovered

* WHEN an enabled extension contributes a valid connector descriptor
* THEN the connector appears with a namespaced id, source, capabilities, auth strategy, and eligibility before its runtime is activated

#### Scenario: Driver advertises unsupported capability

* WHEN a driver response contains a capability absent from its validated descriptor
* THEN the response is rejected and the connector enters a visible degraded/error state

### Requirement: Connector types and capabilities are explicit

The first-version Connector Platform SHALL support typed descriptors for CLI, HTTP API, MCP, Messaging, Workspace, Browser, and Custom connectors. Agent-facing operations SHALL be checked against declared connector capabilities and SHALL NOT infer authority from connector display name or type alone.

#### Scenario: Read-only connector receives write operation

* WHEN a connector declaring only repository-read capabilities receives a pull-request-write operation
* THEN the operation is rejected before driver invocation

#### Scenario: UI filters connector type

* WHEN a user filters Connections by Messaging
* THEN projected messaging connectors and extension Messaging connectors are returned using stable typed metadata

### Requirement: Connector lifecycle is operation-driven and generation-safe

The system SHALL support Discovered, Unconfigured, Ready, Authenticating, Disconnected, Connecting, Connected, Degraded, Reconnecting, AuthorizationExpired, Error, and Disabled lifecycle states. Configure, authenticate, test, connect, disconnect, reconnect, refresh, and uninstall SHALL use stable operations when they may exceed the synchronous budget and SHALL reject stale state transitions.

#### Scenario: Two connect operations race

* WHEN concurrent connect operations target the same connector generation
* THEN only one transition owns the generation and the other receives the current operation/state rather than starting a duplicate driver

#### Scenario: Connector is disabled during reconnect

* WHEN disable commits while a reconnect operation is in flight
* THEN the reconnect is cancelled or its stale completion is ignored and the connector remains Disabled

### Requirement: Authentication strategies are typed and driver-declared

The system SHALL define typed authentication contracts for none, external CLI, API key, OAuth 2.0 authorization code with PKCE, device code, QR pairing, and host-delegated authentication. A connector driver SHALL accept only strategies declared in its descriptor, and UI SHALL render only the corresponding safe flow.

#### Scenario: API-key connector receives OAuth request

* WHEN an authenticate request selects a strategy not declared by the connector
* THEN validation rejects it before driver or browser invocation

#### Scenario: External CLI authentication is incomplete

* WHEN an external-CLI connector's readiness test reports the CLI installed but not authenticated
* THEN state distinguishes installation readiness from authentication readiness and exposes the appropriate action

### Requirement: Connector secrets are stored and used by opaque handle

Connector state, extension packages, manifests, SQLite rows, frontend DTOs, logs, and diagnostics SHALL NOT contain raw secret values. The credential service SHALL return configured/missing/expired/error status and opaque handles; drivers SHALL request scoped secret use through a broker and SHALL receive raw material only when the underlying operation cannot be performed by the host on their behalf.

#### Scenario: Connector list is requested

* WHEN the frontend lists connectors
* THEN each connector exposes credential status but no token, password, API key, authorization header, or secret-store identifier usable outside the broker

#### Scenario: Driver logs request headers

* WHEN a connector driver submits diagnostic metadata containing a secret header
* THEN unified logging redacts the value before persistence/export

### Requirement: Remote authorization prevents token passthrough and redirect abuse

OAuth and remote HTTP connector implementations SHALL validate redirect targets, use PKCE where supported/required, bind tokens to intended resource/audience, preserve state/nonce protections where applicable, refuse token passthrough, and SHALL NOT forward credentials across origins or unauthorized redirects.

#### Scenario: OAuth callback state mismatches

* WHEN a callback contains an unexpected state value
* THEN authentication fails without storing the returned code or token

#### Scenario: Authenticated request redirects cross-origin

* WHEN a connector request receives a redirect to an undeclared origin
* THEN the request stops and no credential is forwarded

### Requirement: Connector operations use Permissions and Hooks

Agent-initiated connector operations SHALL be normalized as connector operations, evaluated by Permissions/rules, and emit connector/tool lifecycle Hooks as applicable. Successful authentication or connection SHALL NOT automatically authorize every Agent-facing capability.

#### Scenario: Connected connector receives high-risk write request

* WHEN an Agent requests a connector write operation requiring Ask
* THEN the operation waits for the current approval flow before driver execution

#### Scenario: Before-send Hook denies outbound message

* WHEN a matching `connector.before_send` Hook returns Deny
* THEN the driver does not transmit the message and the denial is audited

### Requirement: Existing GitHub readiness migrates with compatibility

The current built-in GitHub CLI readiness integration SHALL be represented as a built-in Connector descriptor/driver using the existing readiness semantics. Existing plugin-integration service methods and Tauri commands SHALL delegate to the connector implementation for at least one release and SHALL report compatible configured/readiness results.

#### Scenario: Legacy readiness test runs

* WHEN an existing client invokes the legacy GitHub readiness operation
* THEN it executes through the Connector driver and returns legacy-compatible status without duplicating `gh auth status` logic

#### Scenario: GitHub connector is inspected in unified UI

* WHEN the user opens Connections
* THEN GitHub shows CLI installation/auth/readiness, source Built-in, supported capabilities, last test, and redacted diagnostics

### Requirement: Existing IM connectors are projected without moving ownership

Feishu, Telegram, DingTalk, WeCom, and WeChat connector definitions, state, health, and supported operations SHALL be projected through the published Communications API into Connector Platform. Communications SHALL remain authoritative for persistence, secrets, transports, inbound routing, deduplication, and message delivery.

#### Scenario: IM connector reconnects

* WHEN a user triggers reconnect from the unified Connections page
* THEN Connector Platform delegates to Communications and reflects its generation-safe result rather than running a second transport

#### Scenario: Communications connector receives inbound message

* WHEN an inbound message is delivered
* THEN existing Communications routing remains authoritative while Connector Platform may expose health/trace metadata through projection

### Requirement: MCP connections are projected without duplicating transport ownership

Existing MCP server definitions and state MAY be projected as Connector Type MCP for unified visibility and health. MCP SHALL remain responsible for transport, session negotiation, tool discovery/invocation, limits, credentials, and shutdown.

#### Scenario: MCP row is opened from Connections

* WHEN a projected MCP connector is selected
* THEN the unified view shows summary/health and deep-links to the authoritative MCP configuration page

#### Scenario: User requests MCP reconnect

* WHEN the unified connector action is supported
* THEN Connector Platform delegates to the MCP application API and does not instantiate its own MCP client

### Requirement: Extension connector disable and reload are atomic

Disabling, reloading, rolling back, or uninstalling an extension SHALL atomically update connector descriptor/driver eligibility for new operations. In-flight operations SHALL pin the old runtime generation and follow bounded drain/cancellation; stale completions SHALL NOT resurrect a disabled connector.

#### Scenario: Extension reload changes connector capabilities

* WHEN a new extension version removes or adds capabilities
* THEN the new descriptor becomes current only with the new contribution generation and the UI shows the capability diff before authority expansion

#### Scenario: Connector is active during uninstall

* WHEN uninstall is confirmed while an extension connector is connected
* THEN the system disconnects/drains it according to policy before deleting eligible package state

### Requirement: Connector health is bounded, redacted, and actionable

A Connector health report SHALL contain state, checked-at time, bounded latency, capability readiness, authentication status, recoverable error code, redacted summary, and recommended action. Health checks SHALL be rate-limited, cancellable, and SHALL NOT perform undeclared destructive operations.

#### Scenario: Health test detects expired credential

* WHEN a bounded health test identifies an expired credential
* THEN state becomes AuthorizationExpired and the UI offers re-authentication without exposing the credential

#### Scenario: Driver floods diagnostics

* WHEN a driver returns over-limit logs or health details
* THEN the host truncates/rejects them safely and records a bounded diagnostic
