## MODIFIED Requirements

### Requirement: Native agent tool catalog
The system SHALL provide a fixed, provider-agnostic tool catalog to a native API-based agent's generation, comprising a shell/command-execution tool, a file read/write tool, a content-search tool, a filename-search tool, a scoped file-edit tool, a cross-session memory tool, and the read-only `list_skills`, `load_skill`, and `read_skill_resource` tools. Each tool SHALL be defined once and translated into the request shape required by the session's `interface_format`.

#### Scenario: Tools included in every native generation request
- **WHEN** a chat generation starts for an agent with `launch_kind = api`
- **THEN** the outgoing provider request SHALL declare the shell, file, content-search, filename-search, file-edit, memory, and fixed Skill tools allowed by the active permission mode

#### Scenario: Tool definitions translated per interface format
- **WHEN** the session's `interface_format` is `anthropic`
- **THEN** each tool SHALL be declared using Anthropic's `{name, description, input_schema}` shape
- **WHEN** the session's `interface_format` is `openai-compatible`
- **THEN** each tool SHALL be declared using OpenAI's `{type: "function", function: {name, description, parameters}}` shape

#### Scenario: Skill tools retain fixed schemas
- **WHEN** Skills are added, removed, shadowed, disabled, or migrated
- **THEN** the three fixed Skill tool definitions SHALL remain unchanged
- **AND** Skill-specific names or schemas SHALL NOT be dynamically added to the provider tool catalog

## ADDED Requirements

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

