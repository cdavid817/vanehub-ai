## ADDED Requirements

### Requirement: Remote Terminal diagnostic separation
Remote Terminal lifecycle and persistence diagnostics SHALL use unified logging without turning unified logs into a Terminal transcript store.

#### Scenario: Record remote Terminal lifecycle
- **WHEN** connection acquisition, host verification, authentication, channel creation, keepalive, capture, search, cleanup, or shutdown succeeds or fails
- **THEN** the native runtime SHALL write a bounded redacted diagnostic with safe connection, session, Terminal, run, status, and timing context where available

#### Scenario: Exclude user content from diagnostics
- **WHEN** remote Terminal input, template command text, stdout, stderr, or PTY output is processed
- **THEN** unified logs SHALL omit the raw content and record only safe metadata and redacted summaries

#### Scenario: Capture gap diagnostic
- **WHEN** Terminal output persistence drops content because its bounded queue is full
- **THEN** unified logging SHALL rate-limit a warning containing safe counts and context without including dropped content
