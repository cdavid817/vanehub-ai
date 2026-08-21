## ADDED Requirements

### Requirement: Hook events use versioned typed payloads

The system SHALL define a stable versioned internal event catalog for session, prompt, messages, tool, permission, risk, delegation, and connector lifecycle. Each event SHALL declare its payload schema, admissible handler kinds, admissible decisions, synchrony, default failure behavior, redaction policy, and execution budget.

#### Scenario: Handler returns an invalid decision for an event

* WHEN a handler for `tool.after_execute` returns an input-modification decision not admitted by that event
* THEN the decision is rejected, the event follows its configured failure policy, and the invalid response is audited

#### Scenario: Unknown event version is emitted

* WHEN an emitter or imported configuration references an unsupported event schema version
* THEN dispatch fails with an explicit compatibility diagnostic rather than decoding it as a known security event

### Requirement: Hook dispatch ordering is deterministic

The system SHALL order matching handlers by immutable safety tier, managed/system tier, user/project tier, extension tier, and session tier, then by numeric priority and stable Hook id. Filesystem discovery order, map iteration order, and runtime activation timing SHALL NOT affect the resulting order.

#### Scenario: Two extension Hooks have equal priority

* WHEN two matching extension Hooks have the same priority
* THEN they execute in stable Hook-id order on every platform and restart

#### Scenario: Safety Hook conflicts with extension Hook

* WHEN an immutable safety Hook denies an operation and a later extension Hook requests Continue
* THEN the effective result remains Deny

### Requirement: Hook matching is bounded and context aware

Hook bindings SHALL match only declared fields such as Agent/session id, mode, workspace/project, tool/contribution id, connector id, operation type, risk, and bounded glob/regex predicates. Matcher validation and evaluation SHALL be bounded and SHALL NOT execute arbitrary code.

#### Scenario: Hook matcher contains an invalid regex

* WHEN a Hook definition contains an unsupported or over-limit pattern
* THEN preview/save rejects it without replacing the current active Hook generation

#### Scenario: Session-scoped Hook is dispatched elsewhere

* WHEN an event belongs to a different session than a session-scoped Hook binding
* THEN the Hook does not run

### Requirement: Hook handlers support controlled execution kinds

The system SHALL support built-in, extension-runtime, command, HTTP, MCP-tool, prompt, and read-only Agent handler kinds behind application ports. Every handler SHALL run with explicit timeout, cancellation, input/output limits, provenance, permissions, redaction, and audit. Command, HTTP, prompt, Agent, and MCP handlers SHALL use existing execution/network/model/MCP security boundaries.

#### Scenario: HTTP Hook redirects to an undeclared origin

* WHEN an HTTP handler attempts a redirect outside its configured and approved origin policy
* THEN the request is rejected without forwarding credentials

#### Scenario: Agent Hook attempts a write tool

* WHEN a read-only verification Agent handler attempts a write-capable tool
* THEN the tool request is denied by the Hook handler's enforced tool policy

### Requirement: Hook decisions are event-specific and monotonic for security

The system SHALL support Continue, Deny, Ask, bounded input/output modification, bounded system/message append, and notice emission only where the event schema admits them. Permission and risk Hooks SHALL only preserve or strengthen an existing safety decision and SHALL NOT convert Deny to Ask/Allow or Ask to Allow.

#### Scenario: Permission Hook tries to allow a denied request

* WHEN `permission.requested` receives an existing Deny and a Hook returns Continue or an allow-like result
* THEN the final result remains Deny and the attempted weakening is recorded

#### Scenario: Tool input patch changes protected identity

* WHEN a before-tool Hook patch attempts to change tool id, extension provenance, session id, approval witness, or another protected field
* THEN the patch is rejected before tool execution

### Requirement: Prompt and message transforms preserve protected roles and provenance

Prompt/message Hook transforms SHALL be size-bounded, role-aware, provenance-labelled, and incapable of impersonating protected system messages, tool results, approvals, or user identity. The final prompt trace SHALL identify which Hook appended each accepted fragment.

#### Scenario: Extension appends a protected system message

* WHEN an extension Hook attempts to replace the host-owned system policy or submit an unlabelled protected system message
* THEN the transform is rejected or wrapped in the permitted extension-instruction channel without overwriting host policy

#### Scenario: Multiple Hooks append content

* WHEN several Hooks append admissible content
* THEN the final assembly follows deterministic ordering and retains per-fragment source provenance

### Requirement: Security-critical Hook failures are fail-closed

Hooks marked as enforcement on security-critical events SHALL run synchronously and SHALL fail closed on timeout, protocol failure, invalid response, unavailable required dependency, or circuit-open state. Observational Hooks MAY be configured fail-open, but the failure SHALL remain visible in diagnostics and audit.

#### Scenario: Required PreToolUse Hook times out

* WHEN a fail-closed before-tool Hook exceeds its budget
* THEN the tool does not execute and the system returns Ask or Deny according to the Hook/policy contract

#### Scenario: Observational PostToolUse Hook fails

* WHEN a fail-open observational after-tool Hook fails
* THEN the completed tool result remains available and the Hook failure is recorded without retroactively changing execution

### Requirement: Hook recursion and fan-out are bounded

The system SHALL attach dispatch depth, causal event id, parent handler id, and recursion budget to Hook execution. A handler SHALL NOT recursively trigger itself or unbounded event chains, and per-event handler concurrency/fan-out SHALL be bounded.

#### Scenario: MCP Hook triggers the same Hook event recursively

* WHEN an MCP-tool handler causes an event chain that would invoke the same binding beyond the permitted depth
* THEN recursion is stopped with a stable diagnostic and the outer event follows its failure policy

#### Scenario: Many matching observational Hooks exist

* WHEN matching handler count exceeds the configured event ceiling
* THEN deterministic bounded selection or validation failure occurs rather than unbounded parallel execution

### Requirement: Hook health uses error budgets and circuit breakers

The system SHALL track per-handler latency, timeout, protocol error, invalid-decision, and consecutive-failure state. Crossing configured thresholds SHALL open a circuit for a bounded cooldown, expose the state, and apply the handler's declared fail-open/fail-closed semantics without retry storms.

#### Scenario: Handler repeatedly crashes

* WHEN a Hook handler crosses its failure threshold
* THEN its circuit opens, automatic invocation pauses for the cooldown, and diagnostics expose reset/recovery information

#### Scenario: Fail-closed circuit is open

* WHEN a required enforcement Hook's circuit is open
* THEN matching operations remain blocked or require Ask according to policy; the system does not silently skip the Hook

### Requirement: Hook executions are traceable and redacted

The system SHALL record bounded Hook execution traces containing event/version, handler id/source, matcher result, start/end/duration, decision summary, failure mode, circuit state, related session/tool/connector/operation ids, and redacted payload digests. It SHALL NOT persist raw credentials or unrestricted prompt/tool content.

#### Scenario: User inspects a Hook trace

* WHEN a Hook execution is selected in diagnostics
* THEN the UI can show ordering, duration, decision, and redacted before/after summary without revealing secret fields

#### Scenario: Trace retention limit is reached

* WHEN retained Hook execution data exceeds configured age/count/size limits
* THEN cleanup removes only eligible trace records and preserves required audit evidence

### Requirement: Hooks can be tested without executing the target operation

The system SHALL provide synthetic Hook testing that validates matching, activation, handler execution, admissible decision, timeout, and redaction using an explicitly marked test event. Testing SHALL NOT execute the real target tool, connector operation, file mutation, or grant creation.

#### Scenario: User tests a before-tool Hook

* WHEN the user supplies a synthetic shell-tool event
* THEN the result shows matching handlers and decisions but no shell command is executed

#### Scenario: Test requires an unavailable dependency

* WHEN an extension runtime or MCP dependency needed by the Hook is unavailable
* THEN the test reports the dependency failure without changing the active Hook definition

### Requirement: Claude Code Hook compatibility is versioned and explicit

The system SHALL maintain a versioned compatibility catalog that maps supported Claude Code Hook events and payloads to internal events and handlers. Import SHALL preview mappings, unsupported fields/events, security/failure semantics, and generated definitions before persistence. Unsupported events SHALL NOT be silently ignored.

#### Scenario: Supported PreToolUse configuration is imported

* WHEN a user imports a compatible PreToolUse handler
* THEN preview maps it to the appropriate before-tool/permission events with explicit matcher and failure behavior

#### Scenario: External event is unsupported

* WHEN imported configuration references an event not present in the selected compatibility-catalog version
* THEN preview reports it as unsupported and blocks or excludes it only with explicit user acknowledgement according to import policy

### Requirement: Hook emitters use one published dispatcher contract

Agent Runtime, context compaction, tool execution, delegation, Permissions, and connector flows SHALL emit lifecycle events through the published Hook dispatcher contract. Emitters SHALL NOT load Hook files, invoke extension runtimes, or merge decisions independently.

#### Scenario: Tool execution emits lifecycle events

* WHEN a tool is resolved, approved, executed, and completed or failed
* THEN all corresponding Hook events use the same correlation chain and dispatcher semantics

#### Scenario: Hook subsystem is feature-disabled

* WHEN generalized Hooks are disabled by feature flag
* THEN immutable built-in safety behavior and existing specialized permission behavior remain active while optional generalized handlers are bypassed with an explicit disabled state
