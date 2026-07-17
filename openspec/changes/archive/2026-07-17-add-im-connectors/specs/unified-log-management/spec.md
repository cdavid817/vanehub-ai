## ADDED Requirements

### Requirement: IM connector diagnostics use unified logging
All persistent IM connector diagnostics SHALL use the unified logging service with connector and lifecycle context.

#### Scenario: Persist connector lifecycle event
- **WHEN** a connector starts, stops, reconnects, changes authorization state, or encounters a platform error
- **THEN** the native runtime SHALL write a unified log entry with level, connector id, operation, safe status, retry context, and a concise redacted message

#### Scenario: Preserve settings feedback
- **WHEN** a connector operation produces diagnostic logs
- **THEN** the settings page SHALL still receive a concise service result without reading a feature-local log file

### Requirement: IM-specific sensitive data redaction
The unified logging service SHALL redact connector credentials, authorization artifacts, external identities, and message content before persistence.

#### Scenario: Redact IM diagnostic
- **WHEN** an IM diagnostic contains a token, secret, bearer value, QR payload, authorization code, external chat id, external user id, inbound message, prompt, Agent response, or raw protocol frame
- **THEN** the persisted entry SHALL omit or replace the sensitive value before it is appended to disk

#### Scenario: Log command or request metadata
- **WHEN** the runtime records an IM HTTP, WebSocket, Stream, or polling operation
- **THEN** it SHALL record only safe endpoint classification, status, timing, and redacted error context rather than raw headers or bodies

### Requirement: IM skip and delivery diagnostics
The runtime SHALL record safe diagnostics for ignored inbound events and failed outbound delivery without persisting user content.

#### Scenario: Ignore unsupported event
- **WHEN** the connector ignores a group or unsupported-content event
- **THEN** it SHALL emit at most a redacted debug diagnostic with connector id and safe reason classification

#### Scenario: Final delivery fails
- **WHEN** a connector cannot deliver a completed Agent response
- **THEN** it SHALL persist a redacted error with connector id, internal session/message ids, retry classification, and platform status code when safe

