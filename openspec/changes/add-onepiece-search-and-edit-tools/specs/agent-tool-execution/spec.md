## MODIFIED Requirements

### Requirement: Native agent tool catalog
The system SHALL provide a fixed, provider-agnostic tool catalog to a native API-based agent's generation, comprising a shell/command-execution tool, a file read/write tool, a content-search tool, a filename-search tool, a scoped file-edit tool, and a cross-session memory tool. Each tool SHALL be defined once and translated into the request shape required by the session's `interface_format`.

#### Scenario: Tools included in every native generation request
- **WHEN** a chat generation starts for an agent with `launch_kind = api`
- **THEN** the outgoing provider request SHALL declare the shell, file, content-search, filename-search, file-edit, and memory tools

#### Scenario: Tool definitions translated per interface format
- **WHEN** the session's `interface_format` is `anthropic`
- **THEN** each tool SHALL be declared using Anthropic's `{name, description, input_schema}` shape
- **WHEN** the session's `interface_format` is `openai-compatible`
- **THEN** each tool SHALL be declared using OpenAI's `{type: "function", function: {name, description, parameters}}` shape

### Requirement: Risk-tiered tool approval
The system SHALL classify each tool call's risk by which tool/operation is being invoked, not by inspecting its specific arguments. File-read, content-search, and filename-search operations SHALL execute without requiring user approval. File-write operations, file-edit calls, and shell execution SHALL always require an explicit user approval before executing, regardless of their specific arguments.

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
- **WHEN** the native agent calls the file tool with a write operation
- **THEN** the system SHALL request user approval before executing it, regardless of the file path or content involved

#### Scenario: File edit requires approval
- **WHEN** the native agent calls the file-edit tool
- **THEN** the system SHALL request user approval before executing it, regardless of the file path or replacement content involved

#### Scenario: Shell execution requires approval
- **WHEN** the native agent calls the shell tool
- **THEN** the system SHALL request user approval before executing it, regardless of the specific command

### Requirement: Sandboxed tool execution
The shell tool SHALL execute through a bounded, timed-out, cancellable process execution mechanism rather than an unbounded subprocess call. The file tool SHALL resolve all paths relative to the session's workspace folder and SHALL reject any path that would resolve outside that folder. The content-search and filename-search tools SHALL traverse only within the session's workspace folder, SHALL respect the workspace's `.gitignore`/`.ignore` rules, SHALL skip symbolic links and binary file content rather than following or reading through them, SHALL be cancellable mid-traversal, and SHALL cap their returned results at an explicitly declared limit rather than returning an unbounded result set.

#### Scenario: Shell command exceeds its timeout
- **WHEN** a shell tool call runs longer than the system's fixed timeout
- **THEN** the system SHALL terminate it and report a failure result to the provider

#### Scenario: File path escapes the workspace folder
- **WHEN** the native agent calls the file tool with a path that would resolve outside the session's workspace folder (via traversal or otherwise)
- **THEN** the system SHALL reject the call without accessing the filesystem outside that folder

#### Scenario: File tool unavailable without a workspace folder
- **WHEN** a session has no workspace folder configured
- **THEN** the system SHALL reject any file tool call for that session with a non-retryable failure

#### Scenario: Search traversal respects ignore rules and skips unsafe entries
- **WHEN** the content-search or filename-search tool traverses the workspace folder
- **THEN** the system SHALL exclude paths matched by the workspace's `.gitignore`/`.ignore` rules
- **AND** it SHALL skip symbolic links and binary file content rather than following or reading them

#### Scenario: Search traversal is cancellable
- **WHEN** a user cancels a generation while a content-search or filename-search tool call is traversing the workspace
- **THEN** the system SHALL stop the traversal without completing it

#### Scenario: Search results are capped
- **WHEN** a content-search or filename-search tool call would otherwise match more results than the system's declared limit
- **THEN** the system SHALL truncate the returned results at that limit
- **AND** the result SHALL explicitly indicate that truncation occurred
