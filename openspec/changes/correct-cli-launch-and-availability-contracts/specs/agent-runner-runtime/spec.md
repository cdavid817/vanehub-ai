## MODIFIED Requirements

### Requirement: Runner-scoped security and resource governance
Runner preparation SHALL admit only allowlisted bounded executable specifications, approved cwd and environment keys, and secrets authorized for the selected principal, action, Runner kind, and target. Command construction MUST reject unsafe remote cwd or environment names, unapproved secret forwarding, privileged/container escape intent, and stale authority before side effects.

Control characters MUST be rejected everywhere except tab, carriage return and line feed inside a launch **argument**. Arguments carry user and system text — a composed prompt spans lines by construction — and they reach the operating system as an array, never through a shell, so a line break in one is not an injection vector. Every other control character remains rejected in arguments, and the entire control range remains rejected in the executable, the working directory and environment values, which are identifiers and paths rather than text. NUL in particular MUST stay rejected wherever it can appear: it terminates a C string at the process boundary and would silently truncate the value the caller believes it passed.

The executable SHALL be measured against the bound that applies to a filesystem path rather than the bound that applies to an identifier, since it is resolved to an absolute path before preparation sees it.

A refusal SHALL identify which constraint produced it, so an operator can act on the failure without reproducing it under a debugger.

#### Scenario: Secret is approved only for Local
- **WHEN** an SSH Run requests a secret whose grant covers only Local execution
- **THEN** preparation denies injection and no secret bytes are sent to the remote transport or logs

#### Scenario: Remote command contains unsafe structure
- **WHEN** executable, cwd, argument, or environment metadata cannot be encoded by the bounded remote command contract
- **THEN** preparation rejects it without opening an exec channel

#### Scenario: Concurrent Runner quota is reached
- **WHEN** another Run would exceed the declared global, per-runner, per-profile, output-buffer, or cleanup budget
- **THEN** admission returns a bounded resource-policy error without spawning local or remote work

#### Scenario: Composed prompt is delivered as an argument
- **WHEN** a launch argument contains tab, carriage return or line feed because it carries a composed prompt
- **THEN** preparation SHALL admit it and the Run SHALL proceed

#### Scenario: Argument carries a control character that is not whitespace
- **WHEN** a launch argument contains NUL, an escape sequence, or any other non-whitespace control character
- **THEN** preparation SHALL reject it before side effects

#### Scenario: Executable resolves to a long vendored path
- **WHEN** a managed CLI's executable resolves to an absolute path longer than an identifier bound but within the path bound
- **THEN** preparation SHALL admit it rather than refusing the launch

#### Scenario: Refusal names its constraint
- **WHEN** preparation refuses a launch for any reason above
- **THEN** the refusal SHALL carry the constraint that rejected it, and the unified log SHALL record that constraint alongside the reason code
