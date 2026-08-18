## ADDED Requirements

### Requirement: Agent Runner diagnostics are correlated and redacted
Runner preparation, spawn, disconnect, reconnect, cancel, inspect, cleanup, resource-policy, and recovery diagnostics SHALL use unified logging with operation and canonical Run correlation. Diagnostics SHALL preserve bounded Runner-versus-provider classifications and MUST exclude prompts, provider output, terminal content, raw arguments, environment values, credentials, unrestricted paths, and remote command bodies.

#### Scenario: SSH Runner fails with secret-bearing cause
- **WHEN** native SSH or process infrastructure returns an error containing credential, key path, endpoint user info, environment, or command content
- **THEN** unified persistence receives only redacted allowlisted Runner metadata and a safe classification

#### Scenario: Provider fails on a healthy Runner
- **WHEN** a provider error occurs while Runner transport remains healthy
- **THEN** correlated diagnostics retain separate provider and Runner fields without duplicating raw output

