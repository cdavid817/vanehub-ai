## Purpose

Defines the stable registration, eligibility, permission, lifecycle, readiness, and runtime-boundary rules shared by OnePiece's first-party built-in tools.

## ADDED Requirements

### Requirement: Fixed provider-agnostic handler registry
The system SHALL define each native built-in tool once through a fixed handler registry containing its stable name, versioned input schema, bounded result contract, eligibility predicate, execution-policy compatibility, permission classification, readiness dependencies, and dispatcher. The system SHALL translate eligible definitions into the active provider interface without deriving schemas from runtime inventory.

#### Scenario: Provider interface changes
- **WHEN** the same eligible OnePiece tool catalog is sent through Anthropic and OpenAI-compatible provider interfaces
- **THEN** the system SHALL translate the fixed definitions into each provider's required wire shape without changing their logical names or input contracts

#### Scenario: Runtime inventory changes
- **WHEN** a browser runtime, OCR framework, code runtime, or delegated CLI becomes available or unavailable
- **THEN** the system SHALL recompute eligibility without generating a new provider tool name or inventory-derived schema

### Requirement: New tools are exclusive to OnePiece
The system SHALL make the Browser, Web research, code-execution, OCR, Artifact-publication, and CLI-delegation tools introduced by this change eligible only when the acting Agent has stable id `onepiece`. Display names, launch kinds, capability tags, user-created Agent configuration, and provider identity SHALL NOT grant eligibility.

#### Scenario: OnePiece starts a generation
- **WHEN** a generation starts for stable Agent id `onepiece` and a new tool's readiness and execution-policy predicates pass
- **THEN** the system SHALL include that fixed tool definition in the offered catalog

#### Scenario: Custom API Agent starts a generation
- **WHEN** a generation starts for any user-created API Agent, including one with copied display metadata or capability tags
- **THEN** the system SHALL exclude every new tool introduced by this change

#### Scenario: Forged dispatch bypasses catalog construction
- **WHEN** a caller attempts to dispatch a new tool with an acting Agent id other than `onepiece`
- **THEN** the system SHALL reject the call before approval, native I/O, process launch, browser access, network access, or Artifact access

### Requirement: Eligibility is revalidated at dispatch
The system SHALL revalidate stable Agent identity, session ownership, execution mode, workspace identity, current readiness, current policy, input limits, and cancellation state immediately before executing each built-in tool call. A tool definition's earlier presence in a provider request SHALL NOT authorize later execution.

#### Scenario: Readiness changes during a generation
- **WHEN** a dependency becomes unavailable after the provider received the tool definition but before dispatch
- **THEN** the system SHALL return a bounded unavailable outcome without attempting the native operation

#### Scenario: Session enters a terminal state
- **WHEN** a tool call arrives after its generation has been cancelled, failed, or completed
- **THEN** the system SHALL reject it without performing an effect

### Requirement: Unified permission and approval evaluation
Every effectful built-in tool operation SHALL map to a stable permission action and canonical resource, and SHALL resolve through the existing unified permission and approval service. Readiness and catalog inclusion SHALL NOT imply permission, and tool-specific code SHALL NOT implement an independent approval engine.

#### Scenario: Policy requires approval
- **WHEN** an eligible operation resolves to `Ask`
- **THEN** the operation SHALL remain paused until the unified approval service records an explicit decision or the request becomes stale

#### Scenario: Policy denies an operation
- **WHEN** an eligible operation resolves to `Deny`
- **THEN** the system SHALL return a denial outcome without invoking the handler

#### Scenario: Approval becomes stale
- **WHEN** an approval-bound input hash, capability revision, workspace identity, or target resource changes before execution
- **THEN** the system SHALL reject the stale approval and require a new tool call or approval as applicable

### Requirement: Bounded and cancellable tool lifecycle
Every handler SHALL execute under a shared lifecycle with immutable call identity, deadlines, cancellation propagation, input/output limits, bounded progress events, terminal-state monotonicity, cleanup, and safe error classification. Cancellation SHALL stop owned descendants and SHALL NOT be reported as success.

#### Scenario: User cancels an active tool
- **WHEN** the generation is cancelled while a handler owns a browser, HTTP request, sandbox process, OCR worker, Artifact stream, or delegated CLI process
- **THEN** the system SHALL propagate cancellation, stop owned work, perform bounded cleanup, and emit one cancelled terminal outcome

#### Scenario: Output reaches a hard limit
- **WHEN** a handler reaches its declared output, event, duration, memory, disk, or process limit
- **THEN** the system SHALL stop further production and return an explicit limit-exceeded outcome rather than silently presenting a complete result

### Requirement: Safe persistence and observability
Tool calls and bounded results SHALL remain visible through the existing message activity contract and SHALL emit correlated execution observations. Durable diagnostics SHALL use unified logging and SHALL exclude credentials, hidden reasoning, unrestricted page or file bodies, full external CLI transcripts, raw authorization headers, and unredacted sensitive values.

#### Scenario: Tool call completes
- **WHEN** a new built-in tool reaches a terminal state
- **THEN** chat, task/operation projections, and execution observations SHALL correlate the call using stable ids and safe bounded metadata

#### Scenario: Sensitive input reaches an error path
- **WHEN** a handler fails after receiving a credential-bearing, page, file, prompt, or command input
- **THEN** persisted logs and user-safe errors SHALL contain only redacted summaries and stable error categories

### Requirement: Desktop and Web adapter honesty
The shared frontend service contracts SHALL represent readiness, lifecycle, results, and errors for the new tools in both desktop and Web runtimes. Desktop SHALL route native effects through Tauri-specific adapters and Rust services; Web/mock SHALL perform no native effect and SHALL return deterministic simulation or an explicit desktop-runtime-required outcome.

#### Scenario: React requests native tool state
- **WHEN** a React surface queries or controls a new built-in capability
- **THEN** it SHALL use the shared frontend service boundary and SHALL NOT call Tauri `invoke()` directly

#### Scenario: Web mock receives an effectful call
- **WHEN** Web/mock mode receives an operation that would require local browser, filesystem, process, OCR, Artifact, or CLI access
- **THEN** it SHALL NOT perform or claim the native effect and SHALL return the contract-defined mock or unsupported result

