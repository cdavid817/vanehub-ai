# agent-tool-execution Specification

## Purpose
TBD - created by archiving change add-agent-tool-execution. Update Purpose after archive.
## Requirements
### Requirement: Native agent tool catalog
The system SHALL assemble a fixed, provider-agnostic native API-agent tool catalog from a fixed handler registry. The baseline catalog SHALL continue to comprise a shell/command-execution tool, a file read/write tool, a content-search tool, a filename-search tool, a scoped file-edit tool, a cross-session memory tool, the read-only `list_skills`, `load_skill`, and `read_skill_resource` tools, the `shell_output` and `shell_kill` background-command tools introduced by `add-background-shell-execution`, the `todo_write` task-list tool introduced by `add-agent-task-list`, and the `ask_user_question` clarification tool introduced by `add-agent-user-question`, subject to the existing permission mode and runtime predicates. The registry SHALL additionally contain the fixed Browser, Web research, `code_execution`, OCR, Artifact-publication, `delegate_cli`, and `apply_delegation_changes` handlers introduced by `complete-onepiece-builtin-tool-system`, but those new handlers SHALL be eligible only for stable Agent id `onepiece` and only when their current mode/readiness predicates pass. Each tool SHALL be defined once and translated into the request shape required by the session's `interface_format`; runtime inventory SHALL NOT create dynamic tool names or schemas.

#### Scenario: Tools included in every native generation request
- **WHEN** a chat generation starts for an agent with `launch_kind = api`
- **THEN** the outgoing provider request SHALL declare the baseline shell, file, content-search, filename-search, file-edit, memory, fixed Skill, background-command, task-list, and clarification tools allowed by the active permission mode

#### Scenario: Background-command tools follow the shell tool's permission mode
- **WHEN** the active permission mode excludes the shell tool from the catalog
- **THEN** the outgoing provider request SHALL exclude `shell_kill` for the same reason
- **AND** it MAY still declare the read-only `shell_output` tool, which observes already-approved work without starting or changing any process

#### Scenario: Task-list tool is available in every permission mode
- **WHEN** a chat generation starts in plan mode
- **THEN** the outgoing provider request SHALL still declare `todo_write`, which writes only VaneHub-internal session state and has no workspace, process, or network effect

#### Scenario: Clarification tool is offered only to interactive sessions
- **WHEN** a chat generation starts for an interactive session, including one in plan mode
- **THEN** the outgoing provider request SHALL declare `ask_user_question`
- **WHEN** a generation starts for a Loop worker or verifier, a scheduled-task run, a Plan attempt or repair session, or a delegated Utility attempt
- **THEN** the outgoing provider request SHALL exclude `ask_user_question`

#### Scenario: OnePiece receives eligible extended tools
- **WHEN** a chat generation starts for stable Agent id `onepiece`
- **THEN** the outgoing provider request SHALL additionally declare only the fixed extended tools whose execution-mode and readiness predicates pass

#### Scenario: Custom API Agent does not receive extended tools
- **WHEN** a chat generation starts for a user-created API Agent
- **THEN** the outgoing provider request SHALL exclude every Browser, Web research, code-execution, OCR, Artifact-publication, and CLI-delegation tool introduced by `complete-onepiece-builtin-tool-system` regardless of copied display name, provider, model, or capability tags

#### Scenario: Tool definitions translated per interface format
- **WHEN** the session's `interface_format` is `anthropic`
- **THEN** each eligible tool SHALL be declared using Anthropic's `{name, description, input_schema}` shape
- **WHEN** the session's `interface_format` is `openai-compatible`
- **THEN** each eligible tool SHALL be declared using OpenAI's `{type: "function", function: {name, description, parameters}}` shape

#### Scenario: Skill tools retain fixed schemas
- **WHEN** Skills are added, removed, shadowed, disabled, or migrated
- **THEN** the three fixed Skill tool definitions SHALL remain unchanged
- **AND** Skill-specific names or schemas SHALL NOT be dynamically added to the provider tool catalog

#### Scenario: Extended runtime inventory changes
- **WHEN** a browser runtime, OCR framework, sandbox runtime, or delegated CLI is added, removed, upgraded, or becomes unhealthy
- **THEN** its fixed handler schema SHALL remain stable while catalog eligibility is recomputed

### Requirement: Read-only Skill tool execution
The native tool loop SHALL dispatch `list_skills`, `load_skill`, and `read_skill_resource` through the effective Skill runtime as read-only operations. It SHALL validate every input against the fixed schema and SHALL return bounded structured outcomes through the existing tool-result protocol.

#### Scenario: Valid list request
- **WHEN** the model calls `list_skills` with valid optional filters
- **THEN** the tool loop SHALL return only matching effective Skill metadata available to the active session context

#### Scenario: Valid load request
- **WHEN** the model calls `load_skill` with a canonical id or unambiguous alias
- **THEN** the tool loop SHALL return the bounded load result produced by the effective Skill runtime

#### Scenario: Invalid input rejected
- **WHEN** a Skill tool call contains an unknown field, invalid identifier, unsupported filter, or malformed logical URI
- **THEN** the tool loop SHALL return a validation error without performing a filesystem read

#### Scenario: Tool result persists visibly
- **WHEN** a fixed Skill tool executes during a generation
- **THEN** its call and bounded result metadata SHALL remain visible and persisted using the existing completed-message tool-use behavior
- **AND** diagnostic persistence SHALL NOT copy full Skill instructions or resource bodies into logs

### Requirement: Skill resource sandbox
Skill resource reads SHALL be restricted to indexed readable files inside the selected effective Skill package. Host paths SHALL be canonicalized internally and SHALL NOT grant the model direct filesystem authority.

#### Scenario: Logical URI resolves inside package
- **WHEN** a valid `skill://` resource URI identifies an indexed text resource inside the currently effective package
- **THEN** the tool SHALL read only that resource subject to configured size and character limits

#### Scenario: Symlink escapes package
- **WHEN** a resource path or link resolves outside the effective package boundary
- **THEN** the tool SHALL reject the read before returning content

#### Scenario: Executable resource requested
- **WHEN** an indexed script or executable-like resource is requested in this change
- **THEN** the tool SHALL treat it only as bounded readable text when permitted
- **AND** SHALL NOT execute, import, or register it as a tool

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
The system SHALL classify each tool call's risk by which tool/operation is being invoked, not by inspecting its specific arguments, and SHALL resolve whether it executes immediately, is denied, or requires approval through the unified permission evaluation defined by `permissions-core`. File-read, content-search, and filename-search operations SHALL execute without requiring user approval. File-write operations, file-edit calls, and shell execution SHALL, by default, require an explicit user approval before executing, unless the acting principal's assigned policy resolves the action to `Allow` or `Deny`. Starting a background command SHALL be classified as shell execution and SHALL NOT receive a weaker classification than an equivalent foreground call. Retrieving a background command's output SHALL be classified as a read-only operation and SHALL execute without approval; terminating a background command SHALL execute without approval because it only reduces the effects of already-approved work. Writing the session task list SHALL execute without approval because it changes only VaneHub-internal session state. Asking the user a question SHALL execute without approval, because the user's answer is itself the authorization.

#### Scenario: File read executes without approval
- **WHEN** the native agent calls the file tool with a read operation
- **THEN** the system SHALL execute it immediately without requesting user approval

#### Scenario: Content search executes without approval
- **WHEN** the native agent calls the content-search tool
- **THEN** the system SHALL execute it immediately without requesting user approval, regardless of the search pattern or path involved

#### Scenario: Filename search executes without approval
- **WHEN** the native agent calls the filename-search tool
- **THEN** the system SHALL execute it immediately without requesting user approval, regardless of the pattern or path involved

#### Scenario: File write requires approval
- **WHEN** the native agent calls the file tool with a write operation and no policy resolves the action to `Allow` or `Deny`
- **THEN** the system SHALL request user approval before executing it, regardless of the file path or content involved

#### Scenario: File edit requires approval
- **WHEN** the native agent calls the file-edit tool
- **THEN** the system SHALL request user approval before executing it, regardless of the file path or replacement content involved

#### Scenario: Shell execution requires approval
- **WHEN** the native agent calls the shell tool and no policy resolves the action to `Allow` or `Deny`
- **THEN** the system SHALL request user approval before executing it, regardless of the specific command

#### Scenario: Starting a background command requires the same approval as a foreground command
- **WHEN** the native agent calls the shell tool with background execution requested and no policy resolves the action to `Allow` or `Deny`
- **THEN** the system SHALL request user approval before starting the process
- **AND** the requested action and resource SHALL be the same ones an equivalent foreground shell call would request

#### Scenario: Background output retrieval and termination execute without approval
- **WHEN** the native agent calls `shell_output` or `shell_kill`
- **THEN** the system SHALL execute it immediately without requesting user approval

#### Scenario: Task-list writes execute without approval
- **WHEN** the native agent calls `todo_write`
- **THEN** the system SHALL execute it immediately without requesting user approval

#### Scenario: Asking a question executes without approval
- **WHEN** the native agent calls `ask_user_question`
- **THEN** the system SHALL publish the question without first requesting a separate approval

#### Scenario: A policy-allowed file write or shell call executes without approval
- **WHEN** the acting principal's assigned policy resolves a file-write or shell-execution action to `Allow`
- **THEN** the system SHALL execute it immediately without requesting user approval

#### Scenario: A policy-denied file write or shell call is rejected without prompting
- **WHEN** the acting principal's assigned policy resolves a file-write or shell-execution action to `Deny`
- **THEN** the system SHALL NOT execute it
- **AND** SHALL NOT request user approval

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
The shell tool SHALL execute through a bounded, timed-out, cancellable process execution mechanism rather than an unbounded subprocess call. A foreground shell call's timeout SHALL be the system default unless the call supplies an explicit per-call timeout, which SHALL itself be clamped to a declared maximum; a background command's bound SHALL be its maximum background lifetime rather than the foreground timeout. The file tool SHALL resolve all paths relative to the session's workspace folder and SHALL reject any path that would resolve outside that folder. The file-edit tool SHALL resolve its target path relative to the session's workspace folder and SHALL reject any path that would resolve outside that folder, exactly as the file tool does. Neither the file tool nor the file-edit tool SHALL be able to access a path with any hidden component (a path segment starting with `.`); such a call SHALL be rejected with an explicit error. The content-search and filename-search tools SHALL traverse only within the session's workspace folder, SHALL respect the workspace's `.gitignore`/`.ignore` rules, SHALL skip hidden files and directories (any path component starting with `.`) as well as symbolic links and binary file content rather than following or reading through them, SHALL be cancellable mid-traversal, and SHALL cap their returned results at an explicitly declared limit rather than returning an unbounded result set. Every tool whose description could lead a caller to expect it can reach a hidden path SHALL state in that description that hidden files and directories are unavailable to it.

#### Scenario: Shell command exceeds its timeout
- **WHEN** a foreground shell tool call runs longer than its effective timeout
- **THEN** the system SHALL terminate it and report a failure result to the provider

#### Scenario: Shell call supplies an explicit timeout
- **WHEN** a foreground shell tool call supplies an explicit per-call timeout
- **THEN** the system SHALL apply that timeout after clamping it to the declared maximum
- **AND** a call that supplies no explicit timeout SHALL keep using the system default

#### Scenario: File path escapes the workspace folder
- **WHEN** the native agent calls the file tool with a path that would resolve outside the session's workspace folder (via traversal or otherwise)
- **THEN** the system SHALL reject the call without accessing the filesystem outside that folder

#### Scenario: File-edit path escapes the workspace folder
- **WHEN** the native agent calls the file-edit tool with a path that would resolve outside the session's workspace folder (via traversal or otherwise)
- **THEN** the system SHALL reject the call without accessing the filesystem outside that folder

#### Scenario: File tool unavailable without a workspace folder
- **WHEN** a session has no workspace folder configured
- **THEN** the system SHALL reject any file tool call for that session with a non-retryable failure

#### Scenario: File and file-edit paths with a hidden component are rejected
- **WHEN** the native agent calls the file tool or the file-edit tool with a path that has any component starting with `.` (e.g. `.github/workflows/ci.yml`)
- **THEN** the system SHALL reject the call with an explicit error rather than attempting to access the path

#### Scenario: Search traversal respects ignore rules and skips unsafe or hidden entries
- **WHEN** the content-search or filename-search tool traverses the workspace folder
- **THEN** the system SHALL exclude paths matched by the workspace's `.gitignore`/`.ignore` rules
- **AND** it SHALL exclude hidden files and directories (any path component starting with `.`)
- **AND** it SHALL skip symbolic links and binary file content rather than following or reading them

#### Scenario: Search traversal is cancellable
- **WHEN** a user cancels a generation while a content-search or filename-search tool call is traversing the workspace
- **THEN** the system SHALL stop the traversal without completing it

#### Scenario: Search results are capped
- **WHEN** a content-search or filename-search tool call would otherwise match more results than the system's declared limit
- **THEN** the system SHALL truncate the returned results at that limit
- **AND** the result SHALL explicitly indicate that truncation occurred

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

### Requirement: Bounded tool input and output
The content-search and filename-search tools SHALL cap their returned results at 200 result lines. The file tool's read operation SHALL support `offset`/`limit` paging, SHALL prefix each returned line with its line number, and SHALL cap a single call's output at 2000 lines, 2000 characters per line, and 64KB of total bytes, applying whichever of those three limits is reached first. Every one of these caps SHALL be a hard limit: the content-search tool's `head_limit` parameter and the file tool's `limit` parameter MAY only lower their tool's cap and SHALL NOT raise it above the system default; the filename-search tool's 200-result-line cap is fixed and has no tuning parameter. Before reading a file's content into memory, the content-search tool, the file tool's read operation, and the file-edit tool SHALL check that file's size through filesystem metadata; a file larger than 10MB SHALL be skipped silently by the content-search tool and SHALL be rejected with an explicit error by the file tool's read operation and by the file-edit tool, in both cases without reading its content. The file tool's read operation SHALL detect binary content and refuse to return it with an explicit reason rather than a decoding failure. Whenever a result-count cap or a read cap causes returned content to be incomplete, the tool's output SHALL explicitly state that truncation occurred rather than returning a partial result silently.

#### Scenario: Search results are capped at 200 result lines
- **WHEN** a content-search or filename-search tool call would otherwise produce more than 200 result lines
- **THEN** the system SHALL return only the first 200 result lines
- **AND** the output SHALL explicitly state that results were truncated

#### Scenario: head_limit can only lower the content-search cap
- **WHEN** a content-search tool call supplies a `head_limit` greater than 200
- **THEN** the system SHALL still cap the returned result lines at 200

#### Scenario: File read paginates with offset and limit
- **WHEN** the native agent calls the file tool's read operation with an `offset` and a `limit`
- **THEN** the system SHALL return lines starting at `offset`, up to `limit` lines, each prefixed with its line number

#### Scenario: File read hits its default output caps
- **WHEN** a file read would otherwise return more than 2000 lines, a line longer than 2000 characters, or more than 64KB of total bytes
- **THEN** the system SHALL truncate at whichever of those limits is reached first
- **AND** the output SHALL explicitly state that truncation occurred

#### Scenario: File read's limit parameter can only lower the cap
- **WHEN** the native agent calls the file tool's read operation with a `limit` greater than the system's default line cap
- **THEN** the system SHALL still cap the returned lines at the system's default

#### Scenario: Oversized file is skipped during content search
- **WHEN** the content-search tool encounters a file larger than 10MB while traversing the workspace
- **THEN** the system SHALL detect this from filesystem metadata without reading the file's content
- **AND** it SHALL skip that file silently and continue the search

#### Scenario: Oversized file is rejected on read or edit
- **WHEN** the native agent calls the file tool's read operation or the file-edit tool on a file larger than 10MB
- **THEN** the system SHALL detect this from filesystem metadata before reading the file's content
- **AND** it SHALL reject the call with an explicit error instead of reading it

#### Scenario: Binary file content is refused on read
- **WHEN** the native agent calls the file tool's read operation on a file containing binary content
- **THEN** the system SHALL refuse to return that content
- **AND** it SHALL report an explicit reason rather than a decoding error

### Requirement: Native Agent executes conditional read-only LSP tools
The system SHALL conditionally add `find_definition`, `find_references`, `get_hover`, and `get_diagnostics` to the provider-agnostic native API Agent tool catalog for a configured trusted local workspace. It SHALL execute them as cancellable workspace-read operations with hard input, request, result-count, content, and serialized-output limits, and SHALL preserve their inputs, bounded outputs, and outcomes through the existing visible persisted tool-use lifecycle.

#### Scenario: LSP tools are translated for the provider
- **WHEN** a native API Agent generation starts with LSP available for its current trusted local workspace
- **THEN** the four LSP tools SHALL be declared using the session provider's existing tool-definition translation

#### Scenario: LSP request is cancelled with the generation
- **WHEN** the user stops a generation while an LSP tool request is pending
- **THEN** the runtime SHALL cancel the pending wait and complete bounded protocol cleanup
- **AND** it SHALL NOT continue the request after generation cancellation

#### Scenario: LSP output reaches a hard limit
- **WHEN** accepted LSP locations, diagnostic messages, hover text, previews, or serialized output exceed a declared limit
- **THEN** the tool SHALL return only the bounded result
- **AND** it SHALL explicitly report truncation where applicable

#### Scenario: Session has no eligible local workspace
- **WHEN** the model requests an LSP tool for a session without a trusted eligible local workspace
- **THEN** the runtime SHALL reject the call without starting a server or accessing another workspace

### Requirement: Background shell execution
The shell tool SHALL support an opt-in background execution mode that starts a command, returns an opaque command handle without waiting for the command to finish, and leaves the command running across subsequent tool calls and generations within its owning session. Every background command SHALL be owned by exactly one session and SHALL be subject to a bounded maximum concurrent count per session, a bounded rolling output buffer, and a bounded maximum lifetime. The system SHALL terminate a background command's whole process tree when its lifetime is exhausted, when it is explicitly terminated, when its owning session ends, and when the desktop runtime exits; it SHALL NOT leave an unattended process behind in any of those cases. Background command state SHALL be runtime-only and SHALL NOT be restored after a desktop restart.

#### Scenario: Background command returns a handle immediately
- **WHEN** the native agent calls the shell tool with background execution requested
- **THEN** the system SHALL start the command and return an opaque command handle without waiting for the command to exit
- **AND** the returned result SHALL identify the command as running rather than reporting a completed outcome

#### Scenario: Background command outlives the tool call that started it
- **WHEN** a background command is still running after the tool call that started it has returned
- **THEN** the system SHALL keep the command running and SHALL keep collecting its output for later retrieval within the owning session

#### Scenario: Session concurrency limit is reached
- **WHEN** a session already owns the maximum number of running background commands and the native agent requests another
- **THEN** the system SHALL reject the new request with an explicit limit error
- **AND** it SHALL NOT start another process or terminate an existing one to make room

#### Scenario: Background command exhausts its maximum lifetime
- **WHEN** a background command has been running for longer than the system's maximum background lifetime
- **THEN** the system SHALL terminate its whole process tree
- **AND** subsequent retrieval SHALL report that it was terminated for exceeding its lifetime rather than reporting a normal exit

#### Scenario: Owning session ends while a background command runs
- **WHEN** a session that owns running background commands ends
- **THEN** the system SHALL terminate every one of that session's background command process trees

#### Scenario: Desktop runtime exits while a background command runs
- **WHEN** the desktop runtime exits while background commands are running
- **THEN** the existing process-group and job-object containment SHALL terminate those process trees rather than leaving them attached to no owner

#### Scenario: Background state is not restored after restart
- **WHEN** the desktop runtime starts after an exit that had running background commands
- **THEN** the system SHALL start with no background commands
- **AND** it SHALL NOT present a handle from a previous run as retrievable

### Requirement: Background command output retrieval and termination
The system SHALL expose a read-only `shell_output` tool that returns a background command's output produced since that command's previous retrieval, together with its current lifecycle status and, once it has exited, its exit code. It SHALL expose a `shell_kill` tool that terminates a running background command's whole process tree. Both tools SHALL accept only a command handle owned by the calling session and SHALL reject an unknown handle, a handle owned by another session, and a handle from a previous desktop run with an explicit error rather than a silent empty result. A background command's buffered output SHALL be capped; when the cap is reached the system SHALL discard the oldest buffered output and SHALL explicitly report that output was dropped rather than presenting a contiguous result.

#### Scenario: Output retrieval returns only new output
- **WHEN** the native agent calls `shell_output` for a background command it has retrieved before
- **THEN** the system SHALL return only the output produced since that command's previous retrieval
- **AND** it SHALL report the command's current lifecycle status

#### Scenario: Output retrieval after exit reports the exit code
- **WHEN** the native agent calls `shell_output` for a background command that has exited
- **THEN** the system SHALL return any remaining unretrieved output
- **AND** it SHALL report the command's exit code together with a terminal lifecycle status

#### Scenario: Buffered output exceeds its cap
- **WHEN** a background command produces more output than the system's buffer cap before it is retrieved
- **THEN** the system SHALL retain the most recent output within the cap and discard the oldest
- **AND** the retrieved result SHALL explicitly state that earlier output was dropped

#### Scenario: Explicit termination
- **WHEN** the native agent calls `shell_kill` for a running background command owned by its session
- **THEN** the system SHALL terminate that command's whole process tree
- **AND** subsequent retrieval SHALL report a terminated lifecycle status rather than a normal exit

#### Scenario: Unknown or foreign handle is rejected
- **WHEN** the native agent calls `shell_output` or `shell_kill` with a handle that its session does not own, that never existed, or that belongs to a previous desktop run
- **THEN** the system SHALL reject the call with an explicit error
- **AND** it SHALL NOT return another command's output or terminate another session's process

#### Scenario: Terminating an already-finished command
- **WHEN** the native agent calls `shell_kill` for a background command that has already exited
- **THEN** the system SHALL report that the command was already finished
- **AND** it SHALL NOT report a failure that implies the command is still running

