## ADDED Requirements

### Requirement: LSP diagnostics use unified redacted logging
Language-server discovery, test, process lifecycle, protocol-limit, timeout, cancellation, crash, restart, and shutdown diagnostics SHALL be written through unified logging with bounded safe metadata. Unified logs and telemetry SHALL NOT persist raw LSP payloads, source or hover content, diagnostic messages, raw stderr, environment values, executable arguments, credentials, or private absolute paths.

#### Scenario: Language server crashes
- **WHEN** a managed language-server process exits unexpectedly
- **THEN** unified logging SHALL record level, safe server and language id, lifecycle transition, exit code, restart attempt, and safe reason category
- **AND** it SHALL omit raw stderr and private workspace paths

#### Scenario: Protocol request times out
- **WHEN** an LSP request reaches its bounded deadline
- **THEN** unified logging SHALL record the safe method category, duration, timeout category, server state, and available execution correlation
- **AND** it SHALL not persist request parameters or response content

#### Scenario: Diagnostics are published
- **WHEN** a server publishes source diagnostics
- **THEN** unified logging MAY record bounded counts and severity totals
- **AND** it SHALL NOT persist diagnostic text, related source information, or code excerpts

#### Scenario: Repeated restart failures occur
- **WHEN** a server repeatedly crashes or times out during its restart window
- **THEN** unified logging SHALL rate-limit repeated diagnostics while preserving a safe aggregate count and final restart-exhausted state
