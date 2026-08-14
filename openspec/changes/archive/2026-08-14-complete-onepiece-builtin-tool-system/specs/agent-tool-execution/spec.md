## MODIFIED Requirements

### Requirement: Native agent tool catalog
The system SHALL assemble a fixed, provider-agnostic native API-agent tool catalog from a fixed handler registry. The baseline catalog SHALL continue to comprise a shell/command-execution tool, a file read/write tool, a content-search tool, a filename-search tool, a scoped file-edit tool, a cross-session memory tool, and the read-only `list_skills`, `load_skill`, and `read_skill_resource` tools, subject to the existing permission mode and runtime predicates. The registry SHALL additionally contain the fixed Browser, Web research, `code_execution`, OCR, Artifact-publication, `delegate_cli`, and `apply_delegation_changes` handlers introduced by `complete-onepiece-builtin-tool-system`, but those new handlers SHALL be eligible only for stable Agent id `onepiece` and only when their current mode/readiness predicates pass. Each tool SHALL be defined once and translated into the request shape required by the session's `interface_format`; runtime inventory SHALL NOT create dynamic tool names or schemas.

#### Scenario: Tools included in every native generation request
- **WHEN** a chat generation starts for an agent with `launch_kind = api`
- **THEN** the outgoing provider request SHALL declare the baseline shell, file, content-search, filename-search, file-edit, memory, and fixed Skill tools allowed by the active permission mode

#### Scenario: OnePiece receives eligible extended tools
- **WHEN** a chat generation starts for stable Agent id `onepiece`
- **THEN** the outgoing provider request SHALL additionally declare only the fixed extended tools whose execution-mode and readiness predicates pass

#### Scenario: Custom API Agent does not receive extended tools
- **WHEN** a chat generation starts for a user-created API Agent
- **THEN** the outgoing provider request SHALL exclude every Browser, Web research, code-execution, OCR, Artifact-publication, and CLI-delegation tool introduced by this change regardless of copied display name, provider, model, or capability tags

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

