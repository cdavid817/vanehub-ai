## ADDED Requirements

### Requirement: Native ACP transport behind runtime ports

The desktop runtime SHALL implement ACP stdio behind the existing Agent Runtime application ports with provider-specific infrastructure adapters. React components and Web/mock adapters SHALL NOT spawn local processes or implement native protocol transport.

#### Scenario: UI starts a managed CLI conversation

- **WHEN** a user starts a compatible ACP provider from React
- **THEN** the request SHALL cross the frontend service and Tauri adapter into the native runtime

#### Scenario: Web mock starts a conversation

- **WHEN** the same UI runs in Web/mock mode
- **THEN** it SHALL use deterministic simulated events with a visible simulation indication and no host-side effects

### Requirement: Bounded bidirectional JSON-RPC framing

ACP transport SHALL use UTF-8 newline-delimited JSON-RPC, bounded frames, bounded aggregate queues, bounded pending requests, separate stderr diagnostics, and serialized writes. It MUST support arbitrary byte chunking and continue dispatching inbound requests while awaiting outbound RPC responses.

#### Scenario: Message bytes are split across reads

- **WHEN** valid frames contain multibyte Chinese text split at arbitrary byte boundaries
- **THEN** the decoder SHALL reconstruct equivalent ordered messages without corruption

#### Scenario: Agent requests permission during a prompt

- **WHEN** the host is awaiting a prompt response when the agent sends an inbound request
- **THEN** the reader SHALL dispatch and reply to the inbound request without deadlock

#### Scenario: Inbound resource budget is exceeded

- **WHEN** a frame, queue, JSON nesting level, or pending request count exceeds its declared budget
- **THEN** the runtime SHALL apply controlled backpressure or terminate with a bounded classified error
- **AND** it SHALL NOT drop approvals or final outcomes silently

### Requirement: Initialization and truthful capability negotiation

Every connection SHALL initialize and validate protocol version before session creation. The client SHALL advertise only implemented and policy-permitted capabilities. Unsupported versions SHALL close cleanly; missing optional capabilities SHALL NOT be fabricated.

#### Scenario: Peer version is unsupported

- **WHEN** initialize returns a protocol version unsupported by the host
- **THEN** the runtime SHALL close the connection and report incompatibility without sending a session prompt

#### Scenario: Peer lacks loadSession

- **WHEN** the handshake omits session loading support
- **THEN** resume SHALL be unavailable even when metadata previously suggested it might work

#### Scenario: Client terminal is partially implemented

- **WHEN** the host cannot implement all required terminal lifecycle methods
- **THEN** initialize SHALL not advertise terminal capability

### Requirement: Separate connection session and turn lifecycle

The runtime SHALL model connection, session, and prompt-turn lifecycles separately. Completing a turn SHALL use the prompt response and stop reason rather than process exit. A session SHALL have at most one active prompt turn, and ordinary completion MUST keep a healthy connection reusable.

#### Scenario: Turn completes with process alive

- **WHEN** the agent returns an end-turn response while its process remains running
- **THEN** the runtime SHALL complete the turn and accept a later prompt without waiting for EOF

#### Scenario: Second prompt arrives during a turn

- **WHEN** a caller submits a second prompt to the same busy session
- **THEN** the runtime SHALL reject or explicitly queue it according to the documented policy and SHALL NOT mix turn output

#### Scenario: Agent reports a non-success stop reason

- **WHEN** the prompt stops because of refusal or a limit
- **THEN** the runtime SHALL preserve the stop reason and SHALL NOT report verified task success solely from turn termination

### Requirement: Ordered event normalization and delivery

ACP updates SHALL map to the existing runtime event model with session, turn, seat, tool and provenance identifiers. Text and tool events received before the final response SHALL be projected before the terminal state. Missing usage or reasoning MUST remain absent rather than invented.

#### Scenario: Tool progress and content interleave

- **WHEN** an agent emits text, tool start, tool progress, and tool completion updates
- **THEN** the UI and event store SHALL preserve the correct association and receive order

#### Scenario: Final response follows buffered text

- **WHEN** the decoder receives content and a final response in one read
- **THEN** the runtime SHALL flush preceding content before completing the turn

#### Scenario: Stale update arrives after reconnection

- **WHEN** an event belongs to a retired connection epoch
- **THEN** the runtime SHALL reject or diagnose it without adding it to the current turn

### Requirement: Protocol cancellation and owned process cleanup

Cancellation SHALL send `session/cancel` as a notification, cancel pending interactions, await the original prompt outcome for a bounded grace period, and escalate through the existing owned process-tree termination mechanism when needed. Host-owned proxy terminals SHALL also be stopped and reaped. No unrelated process may be terminated.

#### Scenario: Cooperative cancellation

- **WHEN** an agent responds to cancellation with a cancelled prompt outcome
- **THEN** the runtime SHALL record one cancelled terminal state and may keep the healthy connection

#### Scenario: Agent ignores cancellation

- **WHEN** the cancellation grace period expires without a terminal outcome
- **THEN** the host SHALL terminate and reap owned resources and record forced cancellation with a disconnected binding

#### Scenario: Permission is pending during cancellation

- **WHEN** cancellation occurs while permission requests remain unanswered
- **THEN** the host SHALL respond with the protocol cancelled outcome and reject later approvals

### Requirement: Classified failures without unsafe fallback

Spawn errors, authentication failures, protocol violations, unexpected EOF, broken pipes, timeouts and peer errors SHALL produce bounded classified outcomes. A failed ACP launch or turn MUST NOT silently retry via permissive headless, another provider, another account, or a different installation.

#### Scenario: Protocol connection closes mid-turn

- **WHEN** EOF occurs after the agent may have executed a tool
- **THEN** the runtime SHALL mark the turn interrupted with potentially unknown effects and preserve available evidence

#### Scenario: ACP is unsupported by a binary

- **WHEN** the selected executable rejects its ACP invocation
- **THEN** the runtime SHALL report version incompatibility and SHALL NOT rerun the prompt in unrestricted print mode

### Requirement: Bounded diagnostics and separate time budgets

Connection checks, protocol requests, human interactions, active generation and overall task deadlines SHALL have distinguishable bounded timeout semantics. Diagnostic content SHALL flow through the existing redacted logging boundary with truncation metadata.

#### Scenario: Human approval is awaited

- **WHEN** an agent is waiting on a user decision
- **THEN** the system SHALL show a waiting state and SHALL NOT classify the delay as a malformed protocol or model crash

#### Scenario: Diagnostic stream is unbounded

- **WHEN** an agent continuously writes stderr
- **THEN** the runtime SHALL maintain bounded memory and redacted retained output while preserving a truncation indicator
