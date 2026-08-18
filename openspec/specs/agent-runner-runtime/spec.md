# agent-runner-runtime Specification

## Purpose
Defines how Agent generations execute through provider-neutral Local or SSH runners while preserving secure resource ownership, background page continuity, recovery truthfulness, and runtime-adapter compatibility.
## Requirements
### Requirement: Provider-neutral Runner contract
The Agent runtime SHALL select execution location through a Runner contract that exposes bounded capabilities for prepare, spawn, input, event streaming, cancel, inspect, cleanup, and recover or reconnect where supported. Runner behavior MUST NOT parse provider prompts, output protocols, session identifiers, usage, or provider-specific errors.

#### Scenario: Execute one provider on two runners
- **WHEN** the same stable provider is invoked once with Local and once with an eligible SSH workspace
- **THEN** provider translation remains identical apart from runner-required transport and cwd representation
- **AND** each Runner returns the same normalized transport lifecycle to provider execution orchestration

#### Scenario: Request an unsupported runner capability
- **WHEN** execution requests reconnect, PTY, or another capability the selected Runner does not declare
- **THEN** preparation fails with a bounded runner capability error before provider work or secret injection begins

### Requirement: Compatible Local Runner
The Local Runner SHALL preserve existing CLI generation cwd, approved environment, stdin prompt delivery, structured stdout and stderr streaming, PTY behavior where used, cancellation, process-tree cleanup, exit code, opaque provider resume, operation correlation, and shutdown behavior. Omitted runner selection MUST resolve to Local for compatible existing clients.

#### Scenario: Existing message omits runner selection
- **WHEN** an existing client submits a supported CLI generation without a runner field
- **THEN** the Local Runner executes it with the prior observable provider invocation, streaming, cancellation, usage, resume, and error behavior

#### Scenario: Local cancellation races with exit
- **WHEN** cancellation and natural local process exit occur concurrently
- **THEN** exactly one terminal Run outcome wins and the owned process tree is reaped without duplicate terminal events

### Requirement: SSH Runner reuses native SSH runtime
The SSH Runner SHALL use an existing current SSH profile binding, verified host identity, native-owned credentials, pooled transport, keepalive, and an independent remote exec or PTY channel. It MUST NOT create another SSH transport stack, copy secrets into SQLite or frontend state, or reuse a transport across different profile ids or revisions.

#### Scenario: Start an Agent in a bound SSH workspace
- **WHEN** the user selects SSH for a Session with a current profile revision, matching remote endpoint, trusted host key, available provider command, and approved execution policy
- **THEN** the runtime leases the existing pooled transport, opens an independent channel in the remote cwd, streams normalized events, and records a bounded runner reference

#### Scenario: Reject an unsafe SSH selection
- **WHEN** the profile binding is absent or stale, host identity is untrusted or changed, credentials are unavailable, remote cwd is invalid, or policy denies remote execution
- **THEN** preparation fails closed before authentication or command execution as applicable
- **AND** no fallback to Local occurs

#### Scenario: Shared transport serves independent Runs
- **WHEN** compatible concurrent SSH Runs use one healthy profile revision
- **THEN** the pool establishes at most one authenticated transport and each Run owns cancellation and cleanup of only its independent channel and remote process

### Requirement: Explicit Runner selection and honest availability
Run creation surfaces SHALL list Local and eligible SSH workspace choices through the shared service contract. Docker/Sandbox and future cloud choices SHALL be returned only with truthful unavailable status until an approved implementation supplies their required isolation, image, mount, secret, resource, and cleanup capabilities.

#### Scenario: Select a runner in desktop mode
- **WHEN** the user prepares a CLI Agent Run
- **THEN** the UI shows Local plus only valid SSH bindings with safe host labels and capability state
- **AND** the selected stable runner reference is submitted through the Agent service boundary

#### Scenario: Inspect runners in Web mode
- **WHEN** the browser adapter lists or executes a mock runner
- **THEN** it exposes deterministic simulated Local and SSH states without claiming native credentials, network access, persistence, recovery, tray execution, or OS process control

### Requirement: Background page and window lifecycle
An accepted Agent Run SHALL be owned by the native runtime rather than a mounted React component or visible Session page. Navigating away, closing the Session page, minimizing the window, or hiding the desktop application to the tray MUST NOT cancel the Run; explicit cancel, Session archive/delete policy, timeout, or application quit MAY terminate it according to the canonical owner policy.

#### Scenario: Close a Session page during execution
- **WHEN** a Local or SSH Run is running and the user closes or navigates away from its Session page
- **THEN** execution continues and Mission Control can reopen the authoritative Session and observe its canonical state

#### Scenario: Explicitly quit the application
- **WHEN** the user explicitly quits VaneHub
- **THEN** Local owned process trees are cancelled and reaped and SSH channels receive bounded close or remote-cancel handling
- **AND** the product does not claim that page-background execution survives process exit

### Requirement: Conservative Runner recovery
Each accepted Run SHALL persist a bounded runner kind, stable runner reference, runner capability/recovery classification, and safe inspection witness before execution is presented as running. Startup recovery MUST inspect supported runner state, reconnect only with current authority, mark dead or unverifiable local work interrupted, and MUST NOT replay provider prompts, tool calls, approvals, questions, stdin, or destructive actions.

#### Scenario: Restart after a local process ended
- **WHEN** startup finds a non-terminal Local Run without a verified live owned handle
- **THEN** the Run becomes failed with an interrupted runner reason and is not displayed as running

#### Scenario: Recover a remote Run
- **WHEN** startup finds an SSH Run with a recoverable remote reference and current profile authority
- **THEN** the runtime inspects and reconnects according to bounded policy or records interrupted/attention-required state without replaying work

#### Scenario: Recovery authority changed
- **WHEN** the SSH profile revision, host trust, credential authority, or permission policy differs from the persisted witness
- **THEN** automatic reconnect fails closed and the Run requires explicit user action

### Requirement: Distinct Runner and Provider failures
Failures SHALL identify whether preparation, transport, disconnect, reconnect, cancellation, cleanup, or resource governance belongs to the Runner, while provider protocol, parser, authentication, rate, or model errors remain provider failures. User-visible errors and canonical reason codes SHALL be bounded and MUST NOT expose raw stderr, output, arguments, environment values, credentials, or unrestricted paths.

#### Scenario: SSH transport drops before provider exit
- **WHEN** a remote channel becomes unavailable without a provider terminal event
- **THEN** the Run enters a bounded disconnected or retrying state according to policy and reports a runner transport classification rather than a provider failure

#### Scenario: Provider returns an error over a healthy runner
- **WHEN** transport remains healthy and provider output reports a classified failure
- **THEN** the Run preserves the provider classification and does not relabel it as a runner failure

### Requirement: Runner-scoped security and resource governance
Runner preparation SHALL admit only allowlisted bounded executable specifications, approved cwd and environment keys, and secrets authorized for the selected principal, action, Runner kind, and target. Command construction MUST reject control characters, unsafe remote cwd or environment names, unapproved secret forwarding, privileged/container escape intent, and stale authority before side effects.

#### Scenario: Secret is approved only for Local
- **WHEN** an SSH Run requests a secret whose grant covers only Local execution
- **THEN** preparation denies injection and no secret bytes are sent to the remote transport or logs

#### Scenario: Remote command contains unsafe structure
- **WHEN** executable, cwd, argument, or environment metadata cannot be encoded by the bounded remote command contract
- **THEN** preparation rejects it without opening an exec channel

#### Scenario: Concurrent Runner quota is reached
- **WHEN** another Run would exceed the declared global, per-runner, per-profile, output-buffer, or cleanup budget
- **THEN** admission returns a bounded resource-policy error without spawning local or remote work

### Requirement: Runner contract and integration evidence
Automated coverage SHALL include contract conformance for every Runner, fake local process and SSH transports, Local compatibility fixtures, SSH pool reuse, host/credential negative cases, cancellation races, disconnect/reconnect, restart recovery, cleanup, background UI navigation, desktop integration, Web parity, and deterministic concurrent resource growth.

#### Scenario: Execute the Runner conformance suite
- **WHEN** Local and SSH implementations run against the versioned conformance fixtures
- **THEN** both satisfy common lifecycle, cancellation, cleanup, error, security, and bounded-event invariants
- **AND** implementation-specific recovery claims are tested only when declared supported

