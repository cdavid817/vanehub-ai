## ADDED Requirements

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
The shell tool SHALL execute through a bounded, timed-out, cancellable process execution mechanism rather than an unbounded subprocess call. The file tool SHALL resolve all paths relative to the session's workspace folder and SHALL reject any path that would resolve outside that folder. The file-edit tool SHALL resolve its target path relative to the session's workspace folder and SHALL reject any path that would resolve outside that folder, exactly as the file tool does. Neither the file tool nor the file-edit tool SHALL be able to access a path with any hidden component (a path segment starting with `.`); such a call SHALL be rejected with an explicit error. The content-search and filename-search tools SHALL traverse only within the session's workspace folder, SHALL respect the workspace's `.gitignore`/`.ignore` rules, SHALL skip hidden files and directories (any path component starting with `.`) as well as symbolic links and binary file content rather than following or reading through them, SHALL be cancellable mid-traversal, and SHALL cap their returned results at an explicitly declared limit rather than returning an unbounded result set. Every tool whose description could lead a caller to expect it can reach a hidden path SHALL state in that description that hidden files and directories are unavailable to it.

#### Scenario: Shell command exceeds its timeout
- **WHEN** a shell tool call runs longer than the system's fixed timeout
- **THEN** the system SHALL terminate it and report a failure result to the provider

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
