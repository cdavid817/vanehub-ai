## ADDED Requirements

### Requirement: Feishu IM desktop verification layer
Desktop verification SHALL provide a WebdriverIO layer that launches the native Tauri client with isolated state and deterministically exercises the session IM switch and Feishu delivery boundaries without requiring live credentials for its default run.

#### Scenario: Verify default-off opt-in
- **WHEN** the Feishu IM desktop layer opens a new single-Agent or multi-Agent session
- **THEN** it SHALL observe the information-panel IM switch as off through the real desktop WebView
- **AND** it SHALL verify through the native service boundary that inbound delivery is ineligible

#### Scenario: Verify single-Agent delivery
- **WHEN** the layer enables IM, establishes a fixture Feishu binding, and injects a unique direct-message event
- **THEN** it SHALL observe exactly one Agent turn and one ordered final-response delivery through deterministic fixtures

#### Scenario: Verify multi-Agent routing
- **WHEN** the layer injects messages with a valid seat mention, no seat mention, and an invalid seat mention into an enabled multi-Agent session
- **THEN** it SHALL verify the required stable-seat routing, default routing, and safe rejection behaviors

#### Scenario: Verify resilience boundaries
- **WHEN** the layer exercises duplicate events, disabled sessions, connector interruption, oversized output, malformed events, and application restart
- **THEN** it SHALL verify idempotency, no execution while disabled, safe recovery, ordered chunking, redacted failure evidence, and persisted switch state

### Requirement: Live Feishu qualification is reported separately
Verification results SHALL distinguish deterministic connector fixtures from tests executed against a real Feishu tenant and SHALL never report fixture success as live-platform success.

#### Scenario: Live credentials are unavailable
- **WHEN** no explicitly supplied Feishu test tenant and credentials are available
- **THEN** deterministic desktop scenarios MAY pass
- **AND** live Feishu authentication, event reception, acknowledgement, and reply delivery SHALL be reported as `NOT RUN` or `BLOCKED` with the missing prerequisite

#### Scenario: Live qualification is authorized
- **WHEN** an operator explicitly supplies a Feishu test tenant, application credentials, and a permitted test chat
- **THEN** the qualification SHALL exercise authentication, connection lifecycle, direct-message receipt, duplicate delivery, single-Agent reply, multi-Agent routing, and outbound reply
- **AND** retained evidence SHALL exclude credentials, external identifiers, and message contents

