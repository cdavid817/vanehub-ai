## ADDED Requirements

### Requirement: Active provider delivery baseline

Qwen, Kimi, Qoder, CodeBuddy, Copilot, and Cursor adapters SHALL each implement installation identity, explicit login guidance, native terminal launch, ACP session launch, text exchange, tool event normalization, applicable permission handling, cancellation, classified errors, and truthful optional capabilities. Their ids SHALL NOT vary by protocol. Missing infrastructure or disabled capabilities alone SHALL NOT count as completed integration.

#### Scenario: A supported active provider is selected

- **WHEN** a supported installation passes preflight for managed conversation
- **THEN** the provider SHALL create an ACP session and exchange messages through the shared runtime

#### Scenario: Optional resume is unavailable

- **WHEN** a supported installation does not expose verified resume support
- **THEN** the adapter SHALL offer new sessions and explicitly reject resume without claiming full resume compatibility

### Requirement: Qwen Code isolated adapter contract

The `qwen-code` adapter SHALL use the reviewed `qwen` executable and `--acp` invocation for ACP, and SHALL own its grammar, version profile, authentication assumptions, events, and permissions independently of Gemini. Any enabled headless path MUST have separate verified input/output and safety contracts.

#### Scenario: Start Qwen managed conversation

- **WHEN** Qwen is launched through the ACP transport
- **THEN** the adapter SHALL construct reviewed structured arguments containing `--acp` and keep prompts on the protocol channel

#### Scenario: Headless output is structured

- **WHEN** a Qwen version supports structured stdout but bidirectional input has not been verified
- **THEN** the adapter SHALL NOT infer a bidirectional control channel from the output format

### Requirement: Kimi distribution-aware adapter contract

The `kimi-cli` adapter SHALL distinguish reviewed current and legacy distributions even when both expose `kimi`. ACP SHALL use the verified `kimi acp` contract. Dependencies, configuration references, and resume compatibility SHALL be evaluated per installation distribution. Migration MUST remain a separate explicitly approved operation outside automatic detection.

#### Scenario: Current Kimi is selected

- **WHEN** the selected Kimi distribution is compatible with its adapter profile
- **THEN** the runtime SHALL use its verified invocation and dependency requirements without imposing legacy Python requirements on it

#### Scenario: Print mode cannot satisfy approvals

- **WHEN** a selected Kimi print mode does not support the requested human approval policy
- **THEN** the runtime SHALL refuse it rather than treating print execution as an approval-preserving fallback

#### Scenario: History belongs to a different distribution

- **WHEN** a resume binding was created by a Kimi distribution without verified compatibility with the selected one
- **THEN** the runtime SHALL preserve history and refuse incompatible resume

### Requirement: Qoder scoped ACP adapter

The `qoder-cli` adapter SHALL use reviewed `qoder --acp` invocation and verified login/profile rules. Any historical command alias SHALL require explicit identity and grammar verification. Policy and configuration applied at process start SHALL remain isolated to its execution binding.

#### Scenario: Start Qoder with a profile

- **WHEN** Qoder is launched with a selected account profile and permission policy
- **THEN** the adapter SHALL start a dedicated scoped process using reviewed options
- **AND** it SHALL NOT silently enable bypass permissions

#### Scenario: Unsupported Qoder platform is encountered

- **WHEN** the detected platform or architecture has no supported distribution profile
- **THEN** the provider SHALL report the precise incompatibility instead of falling back to another execution environment silently

### Requirement: CodeBuddy environment and proxy adapter

The `codebuddy-code` adapter SHALL use reviewed `codebuddy --acp`, map explicitly chosen account environments, and only advertise implemented client proxy capabilities. Provider-owned team events SHALL retain parent-seat provenance and SHALL NOT create independent VaneHub seats or duplicate usage.

#### Scenario: CodeBuddy delegates file access

- **WHEN** CodeBuddy sends a file request after the client advertises file capabilities
- **THEN** the operation SHALL pass through the shared permission-controlled filesystem API

#### Scenario: Client proxy is not implemented

- **WHEN** the host has not implemented all required terminal proxy methods
- **THEN** the handshake SHALL NOT advertise terminal support

#### Scenario: CodeBuddy reports internal team activity

- **WHEN** the agent reports a provider-owned subagent update
- **THEN** the UI SHALL associate it with the owning VaneHub seat and avoid double-counting usage

### Requirement: Independent Copilot CLI adapter

The `copilot-cli` adapter SHALL target the independent `copilot` CLI and use reviewed `--acp --stdio`. It SHALL NOT invoke the historical GitHub CLI extension as an equivalent adapter. Process-level tool and reasoning options MUST be scoped to a dedicated execution binding.

#### Scenario: Start Copilot through stdio

- **WHEN** a Copilot ACP session is requested
- **THEN** the adapter SHALL launch the standalone executable with stdio transport and no TCP listener

#### Scenario: Two tasks require different tool filters

- **WHEN** two Copilot tasks have different startup-level policy settings
- **THEN** the runtime SHALL use separate processes or reject unsupported reuse
- **AND** one task SHALL NOT inherit the other task filter or credentials

### Requirement: Cursor identity and blocking extensions

The `cursor-agent-cli` adapter SHALL validate Cursor identity before using the reviewed `agent acp` contract. It SHALL implement the reviewed blocking `cursor/ask_question` and `cursor/create_plan` requests with user-visible bounded interactions and correct responses. Notification-only extensions SHALL NOT receive JSON-RPC responses.

#### Scenario: Cursor asks a question

- **WHEN** a valid `cursor/ask_question` request arrives
- **THEN** the host SHALL show the question and send a matching validated response or explicit cancellation before its deadline

#### Scenario: Cursor requests plan approval

- **WHEN** a valid `cursor/create_plan` request arrives
- **THEN** the host SHALL require the applicable explicit decision and SHALL NOT infer approval from closing its view

#### Scenario: Cursor sends a notification

- **WHEN** a reviewed todo or task notification arrives without an RPC id
- **THEN** the host SHALL project safe content without replying as though it were a request

#### Scenario: Unknown blocking extension arrives

- **WHEN** Cursor requests an unsupported extension method
- **THEN** the host SHALL return a classified protocol error or documented rejection rather than wait indefinitely

### Requirement: Adapter evidence and version qualification

Each adapter SHALL record the reviewed upstream sources, verification date, exact tested binary version/distribution/platform, fixture provenance, supported feature set, and unresolved limitations. Documentation evidence and synthetic conformance tests SHALL NOT be labeled real CLI interoperability results.

#### Scenario: Only synthetic tests pass

- **WHEN** a provider passes fake ACP tests but has no real binary smoke result
- **THEN** the report SHALL distinguish contract-tested from live-verified and retain the live gate as not run or blocked

#### Scenario: An upstream command differs

- **WHEN** the target binary help or handshake contradicts the documented profile
- **THEN** implementation SHALL update the profile and fixture evidence or report incompatibility rather than guess flags
