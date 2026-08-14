## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Native agent tool catalog
The system SHALL assemble a fixed, provider-agnostic native API-agent tool catalog from a fixed handler registry. The baseline catalog SHALL continue to comprise a shell/command-execution tool, a file read/write tool, a content-search tool, a filename-search tool, a scoped file-edit tool, a cross-session memory tool, the read-only `list_skills`, `load_skill`, and `read_skill_resource` tools, and the `shell_output` and `shell_kill` background-command tools introduced by `add-background-shell-execution`, subject to the existing permission mode and runtime predicates. The registry SHALL additionally contain the fixed Browser, Web research, `code_execution`, OCR, Artifact-publication, `delegate_cli`, and `apply_delegation_changes` handlers introduced by `complete-onepiece-builtin-tool-system`, but those new handlers SHALL be eligible only for stable Agent id `onepiece` and only when their current mode/readiness predicates pass. Each tool SHALL be defined once and translated into the request shape required by the session's `interface_format`; runtime inventory SHALL NOT create dynamic tool names or schemas.

#### Scenario: Tools included in every native generation request
- **WHEN** a chat generation starts for an agent with `launch_kind = api`
- **THEN** the outgoing provider request SHALL declare the baseline shell, file, content-search, filename-search, file-edit, memory, fixed Skill, and background-command tools allowed by the active permission mode

#### Scenario: Background-command tools follow the shell tool's permission mode
- **WHEN** the active permission mode excludes the shell tool from the catalog
- **THEN** the outgoing provider request SHALL exclude `shell_kill` for the same reason
- **AND** it MAY still declare the read-only `shell_output` tool, which observes already-approved work without starting or changing any process

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

### Requirement: Risk-tiered tool approval
The system SHALL classify each tool call's risk by which tool/operation is being invoked, not by inspecting its specific arguments, and SHALL resolve whether it executes immediately, is denied, or requires approval through the unified permission evaluation defined by `permissions-core`. File-read, content-search, and filename-search operations SHALL execute without requiring user approval. File-write operations, file-edit calls, and shell execution SHALL, by default, require an explicit user approval before executing, unless the acting principal's assigned policy resolves the action to `Allow` or `Deny`. Starting a background command SHALL be classified as shell execution and SHALL NOT receive a weaker classification than an equivalent foreground call. Retrieving a background command's output SHALL be classified as a read-only operation and SHALL execute without approval; terminating a background command SHALL execute without approval because it only reduces the effects of already-approved work.

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

#### Scenario: A policy-allowed file write or shell call executes without approval
- **WHEN** the acting principal's assigned policy resolves a file-write or shell-execution action to `Allow`
- **THEN** the system SHALL execute it immediately without requesting user approval

#### Scenario: A policy-denied file write or shell call is rejected without prompting
- **WHEN** the acting principal's assigned policy resolves a file-write or shell-execution action to `Deny`
- **THEN** the system SHALL NOT execute it
- **AND** SHALL NOT request user approval

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
