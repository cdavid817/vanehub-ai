## ADDED Requirements

### Requirement: Execution correlation in unified logs
Unified diagnostic and operation logs SHALL include safe run, trace, and span correlation fields when an execution context is available.

#### Scenario: Agent process diagnostic is written
- **WHEN** a managed Agent process emits a lifecycle or failure diagnostic within an execution run
- **THEN** the persisted log SHALL include run id, trace id, span id, session id, Agent id, and operation id when available
- **AND** existing level, category, message, timestamp, and redaction behavior SHALL remain available

#### Scenario: Diagnostic has no execution context
- **WHEN** a native diagnostic occurs outside an execution run
- **THEN** the unified logging service SHALL persist it without fabricating correlation fields

### Requirement: Redaction before every telemetry sink
Diagnostic and operation data SHALL pass the unified sensitive-information policy before local log persistence, local trace persistence, or OpenTelemetry export.

#### Scenario: Correlated diagnostic contains sensitive values
- **WHEN** a correlated log message or context contains a credential, prompt, model response, tool payload, MCP payload, private path, header, or environment value
- **THEN** the sensitive value SHALL be omitted or redacted before it reaches any enabled sink

#### Scenario: Content capture is enabled
- **WHEN** redacted content capture is explicitly enabled
- **THEN** the same redaction policy SHALL apply before bounded summaries are stored or exported
- **AND** unified logs SHALL NOT become a raw replay-content store

### Requirement: Optional OpenTelemetry log bridge
The native logging infrastructure SHALL preserve the configured unified local log while optionally exporting correlated log records through the OpenTelemetry pipeline.

#### Scenario: OTLP log export is disabled
- **WHEN** telemetry export is disabled
- **THEN** local unified logging SHALL continue with no Collector or network dependency

#### Scenario: OTLP log export fails
- **WHEN** the OpenTelemetry log exporter fails
- **THEN** local unified logging SHALL remain available
- **AND** the runtime SHALL rate-limit a redacted local exporter diagnostic and prevent recursive export attempts for that diagnostic

