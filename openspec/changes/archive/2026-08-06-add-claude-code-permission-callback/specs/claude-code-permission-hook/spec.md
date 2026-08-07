## ADDED Requirements

### Requirement: PreToolUse requests resolve through the existing decision pipeline
The system SHALL translate every `PreToolUse` request the hook wrapper forwards into an `Action`/`Resource` pair and resolve it through the same `evaluate()`/`ApprovalBroker` pipeline used for native API agents, without a separate decision engine.

#### Scenario: Mapped tool call is evaluated identically to a native agent's
- **WHEN** the loopback server receives a `PreToolUse` request for a tool that maps to an existing `Action`
- **THEN** the system SHALL resolve it to `Allow`, `Deny`, or `Ask` using the same policy templates, grants, and audit trail a native agent's equivalent action would use

#### Scenario: Ask resolution blocks until a human decision or timeout
- **WHEN** the resolved effect is `Ask`
- **THEN** the system SHALL create a pending approval visible in the existing `ApprovalCard` UI and hold the HTTP response until it is resolved by a human decision or by the existing timeout sweep

### Requirement: Tool-to-Action mapping is explicit and partial
The system SHALL match only `Bash`, `Edit`, `Write`, `Read`, `Glob`, `Grep`, and MCP tool names (`mcp__*`) in the hook's matcher configuration, mapping them respectively to `shell.exec`, `file.write`, `file.write`, `file.read`, `file.read`, `file.read`, and `mcp.tool`, and SHALL NOT match any other tool name.

#### Scenario: Unmapped tool is not intercepted
- **WHEN** Claude Code invokes a tool with no configured mapping (for example `WebFetch`)
- **THEN** the hook SHALL NOT fire for that call
- **AND** Claude Code's native behavior for that tool SHALL be unaffected

#### Scenario: MCP tool call is floored at Ask
- **WHEN** a mapped MCP tool call reaches the decision pipeline
- **THEN** it SHALL resolve to `Ask` regardless of the `claude-code` principal's assigned template, matching `permissions-core`'s existing MCP floor

### Requirement: Loopback server is authenticated and bound to localhost only
The system SHALL bind the permission-hook HTTP server to a loopback address only, SHALL generate a random bearer token once per application launch, SHALL require every request to present that token, and SHALL reject any request missing or presenting an incorrect token.

#### Scenario: Request without a valid token is rejected
- **WHEN** a request to the loopback server does not present the current launch's token
- **THEN** the system SHALL reject the request without evaluating any permission decision

#### Scenario: Token and port are regenerated each launch
- **WHEN** the desktop application starts
- **THEN** the system SHALL generate a new random port and token for the permission-hook server and SHALL write both to a local discovery location the hook wrapper reads

### Requirement: Unreachable server falls back to a risk-tiered, asymmetric default
The system SHALL, when the hook wrapper cannot reach the loopback server for any reason (not running, connection refused, timeout, or missing discovery data), resolve only a fixed allowlist of read-only tools (`Read`, `Glob`, `Grep`) to `Allow`, and SHALL resolve every other tool to `Deny` in that condition.

#### Scenario: Server unreachable and tool is on the read-only allowlist
- **WHEN** the hook wrapper cannot reach the loopback server and the requested tool is `Read`, `Glob`, or `Grep`
- **THEN** the wrapper SHALL resolve the request to `Allow` without contacting VaneHub

#### Scenario: Server unreachable and tool is not on the allowlist
- **WHEN** the hook wrapper cannot reach the loopback server and the requested tool is not on the read-only allowlist, including tools the wrapper does not recognize
- **THEN** the wrapper SHALL resolve the request to `Deny`

### Requirement: Malformed hook payloads fail closed
The system SHALL resolve a `PreToolUse` request to `Deny` if the hook wrapper cannot parse the request or response payload, and SHALL NOT crash or hang the calling Claude Code process as a result.

#### Scenario: Unparseable stdin payload
- **WHEN** the hook wrapper receives a stdin payload it cannot parse
- **THEN** the wrapper SHALL exit denying the tool call
- **AND** SHALL NOT hang waiting for further input

#### Scenario: Unparseable server response
- **WHEN** the loopback server returns a response the wrapper cannot parse
- **THEN** the wrapper SHALL treat the request as denied

#### Scenario: Malformed response does not use the offline allowlist
- **WHEN** the loopback server responds but the wrapper cannot parse the response body
- **THEN** the wrapper SHALL resolve the request to `Deny` regardless of whether the requested tool is on the read-only offline-fallback allowlist

### Requirement: Wrapper timeout is bounded between VaneHub's approval timeout and Claude Code's hook ceiling
The system SHALL bound the hook wrapper's own client-side wait to a duration longer than the pending-approval timeout the native decision pipeline already enforces, and shorter than Claude Code's own hook timeout ceiling.

#### Scenario: Legitimate pending approval is not cut off early
- **WHEN** a mapped action resolves to `Ask` and a human has not yet responded
- **THEN** the wrapper SHALL continue waiting at least until the native pending-approval timeout has had a chance to resolve the request

#### Scenario: Wrapper always resolves before Claude Code's own ceiling
- **WHEN** the wrapper's own wait duration elapses without a response
- **THEN** the wrapper SHALL resolve the request as denied rather than waiting indefinitely

### Requirement: Web runtime has no permission-hook surface
The Web/mock runtime SHALL NOT start a loopback server, write a discovery file, or otherwise simulate this capability, since it has no user-facing surface of its own.

#### Scenario: Web/mock runtime is unaffected
- **WHEN** the application runs in Web/mock mode
- **THEN** no loopback server, discovery file, or hook wrapper behavior SHALL be present or simulated
