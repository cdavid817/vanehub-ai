## ADDED Requirements

### Requirement: Native agent tool catalog
The system SHALL provide exactly two tools to a native API-based agent's generation: a shell/command-execution tool and a file read/write tool. Each tool SHALL be defined once, provider-agnostically, and translated into the request shape required by the session's `interface_format`.

#### Scenario: Tools included in every native generation request
- **WHEN** a chat generation starts for an agent with `launch_kind = api`
- **THEN** the outgoing provider request SHALL declare both the shell tool and the file read/write tool

#### Scenario: Tool definitions translated per interface format
- **WHEN** the session's `interface_format` is `anthropic`
- **THEN** each tool SHALL be declared using Anthropic's `{name, description, input_schema}` shape
- **WHEN** the session's `interface_format` is `openai-compatible`
- **THEN** each tool SHALL be declared using OpenAI's `{type: "function", function: {name, description, parameters}}` shape

### Requirement: Multi-turn tool-use loop
The system SHALL, when a provider response requests one or more tool calls, execute those calls and send their results back to the provider as a new turn, repeating until the provider returns a response with no further tool calls, up to a fixed maximum number of round trips per user message.

#### Scenario: Single tool call resolves and continues the conversation
- **WHEN** a provider response contains one tool call
- **THEN** the system SHALL execute it, send the result back to the provider, and continue the generation using the provider's next response

#### Scenario: Final response has no tool calls
- **WHEN** a provider response contains no tool calls
- **THEN** the system SHALL treat its content as the terminal response for that user message, exactly as a tool-free generation does today

#### Scenario: Round-trip limit exceeded
- **WHEN** the number of tool-call round trips for one user message exceeds the system's fixed maximum
- **THEN** the system SHALL end the generation with a non-retryable failure rather than continuing to call the provider

### Requirement: Risk-tiered tool approval
The system SHALL classify each tool call's risk by which tool/operation is being invoked, not by inspecting its specific arguments. File-read operations SHALL execute without requiring user approval. File-write operations and shell execution SHALL always require an explicit user approval before executing, regardless of their specific arguments.

#### Scenario: File read executes without approval
- **WHEN** the native agent calls the file tool with a read operation
- **THEN** the system SHALL execute it immediately without requesting user approval

#### Scenario: File write requires approval
- **WHEN** the native agent calls the file tool with a write operation
- **THEN** the system SHALL request user approval before executing it, regardless of the file path or content involved

#### Scenario: Shell execution requires approval
- **WHEN** the native agent calls the shell tool
- **THEN** the system SHALL request user approval before executing it, regardless of the specific command

### Requirement: Approval request and resolution
When a tool call requires approval, the system SHALL pause that tool call, present the tool name and its input to the user, and wait for an explicit approve or deny decision before proceeding. The system SHALL NOT execute a tool call awaiting approval before a decision is received.

#### Scenario: Approval requested
- **WHEN** a tool call requiring approval is reached
- **THEN** the system SHALL emit an event carrying the tool name and input, signaling that approval is pending

#### Scenario: User approves
- **WHEN** the user approves a pending tool call
- **THEN** the system SHALL execute it and continue the generation with its result

#### Scenario: User denies
- **WHEN** the user denies a pending tool call
- **THEN** the system SHALL NOT execute it
- **AND** the system SHALL report the denial back to the provider as the tool's result, allowing the generation to continue

#### Scenario: Generation stopped while a tool call awaits approval
- **WHEN** a user stops a generation while a tool call is awaiting approval
- **THEN** the system SHALL end the generation without executing that tool call

### Requirement: Sandboxed tool execution
The shell tool SHALL execute through a bounded, timed-out, cancellable process execution mechanism rather than an unbounded subprocess call. The file tool SHALL resolve all paths relative to the session's workspace folder and SHALL reject any path that would resolve outside that folder.

#### Scenario: Shell command exceeds its timeout
- **WHEN** a shell tool call runs longer than the system's fixed timeout
- **THEN** the system SHALL terminate it and report a failure result to the provider

#### Scenario: File path escapes the workspace folder
- **WHEN** the native agent calls the file tool with a path that would resolve outside the session's workspace folder (via traversal or otherwise)
- **THEN** the system SHALL reject the call without accessing the filesystem outside that folder

#### Scenario: File tool unavailable without a workspace folder
- **WHEN** a session has no workspace folder configured
- **THEN** the system SHALL reject any file tool call for that session with a non-retryable failure

### Requirement: Tool use is visible and persisted on the completed message
Every tool call executed during a generation's tool-use loop SHALL be recorded on that generation's completed assistant message, visible in the chat transcript, regardless of how many round trips occurred before the final response.

#### Scenario: Completed message lists every tool call
- **WHEN** a generation's tool-use loop executes two tool calls before producing its final response
- **THEN** the persisted assistant message SHALL include both tool calls with their inputs, outputs, and outcomes

### Requirement: Web runtime tool-use parity
The Web/mock runtime SHALL simulate a deterministic tool-call, approval, and result sequence for API-based agent sessions without performing real process execution or filesystem access.

#### Scenario: Web mock tool call
- **WHEN** a user sends a message to an API-based agent's session in Web mode
- **THEN** the Web adapter SHALL emit a deterministic simulated tool-call sequence through the same event contract the desktop runtime uses
- **AND** it SHALL NOT execute a real shell command or access the real filesystem
