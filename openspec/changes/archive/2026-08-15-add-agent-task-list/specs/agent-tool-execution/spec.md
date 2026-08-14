## MODIFIED Requirements

### Requirement: Native agent tool catalog
The system SHALL assemble a fixed, provider-agnostic native API-agent tool catalog from a fixed handler registry. The baseline catalog SHALL continue to comprise a shell/command-execution tool, a file read/write tool, a content-search tool, a filename-search tool, a scoped file-edit tool, a cross-session memory tool, the read-only `list_skills`, `load_skill`, and `read_skill_resource` tools, the `shell_output` and `shell_kill` background-command tools introduced by `add-background-shell-execution`, and the `todo_write` task-list tool introduced by `add-agent-task-list`, subject to the existing permission mode and runtime predicates. The registry SHALL additionally contain the fixed Browser, Web research, `code_execution`, OCR, Artifact-publication, `delegate_cli`, and `apply_delegation_changes` handlers introduced by `complete-onepiece-builtin-tool-system`, but those new handlers SHALL be eligible only for stable Agent id `onepiece` and only when their current mode/readiness predicates pass. Each tool SHALL be defined once and translated into the request shape required by the session's `interface_format`; runtime inventory SHALL NOT create dynamic tool names or schemas.

#### Scenario: Tools included in every native generation request
- **WHEN** a chat generation starts for an agent with `launch_kind = api`
- **THEN** the outgoing provider request SHALL declare the baseline shell, file, content-search, filename-search, file-edit, memory, fixed Skill, background-command, and task-list tools allowed by the active permission mode

#### Scenario: Background-command tools follow the shell tool's permission mode
- **WHEN** the active permission mode excludes the shell tool from the catalog
- **THEN** the outgoing provider request SHALL exclude `shell_kill` for the same reason
- **AND** it MAY still declare the read-only `shell_output` tool, which observes already-approved work without starting or changing any process

#### Scenario: Task-list tool is available in every permission mode
- **WHEN** a chat generation starts in plan mode
- **THEN** the outgoing provider request SHALL still declare `todo_write`, which writes only VaneHub-internal session state and has no workspace, process, or network effect

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
The system SHALL classify each tool call's risk by which tool/operation is being invoked, not by inspecting its specific arguments, and SHALL resolve whether it executes immediately, is denied, or requires approval through the unified permission evaluation defined by `permissions-core`. File-read, content-search, and filename-search operations SHALL execute without requiring user approval. File-write operations, file-edit calls, and shell execution SHALL, by default, require an explicit user approval before executing, unless the acting principal's assigned policy resolves the action to `Allow` or `Deny`. Starting a background command SHALL be classified as shell execution and SHALL NOT receive a weaker classification than an equivalent foreground call. Retrieving a background command's output SHALL be classified as a read-only operation and SHALL execute without approval; terminating a background command SHALL execute without approval because it only reduces the effects of already-approved work. Writing the session task list SHALL execute without approval because it changes only VaneHub-internal session state.

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

#### Scenario: A policy-allowed file write or shell call executes without approval
- **WHEN** the acting principal's assigned policy resolves a file-write or shell-execution action to `Allow`
- **THEN** the system SHALL execute it immediately without requesting user approval

#### Scenario: A policy-denied file write or shell call is rejected without prompting
- **WHEN** the acting principal's assigned policy resolves a file-write or shell-execution action to `Deny`
- **THEN** the system SHALL NOT execute it
- **AND** SHALL NOT request user approval
