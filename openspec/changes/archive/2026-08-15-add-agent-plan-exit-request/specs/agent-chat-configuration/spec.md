## MODIFIED Requirements

### Requirement: Plan mode restricts a native API agent to read-only tools
The system SHALL, when the session's permission mode is plan mode, offer a native API agent only tools that cannot modify the user's system, execute arbitrary code, start an external delegated Agent, apply delegated changes, publish new Artifacts, or call an arbitrary network or tool server. Read-only fixed Skill tools SHALL remain available, as SHALL configured read-only LSP queries against an explicitly trusted local workspace. For OnePiece, bounded reads of existing Artifacts and local OCR extraction that does not publish a derived Artifact MAY remain available when their readiness predicates pass. The plan-mode catalog SHALL additionally offer the tool by which the model requests to leave plan mode, which modifies nothing and takes effect only on explicit user approval. The system SHALL reject any attempt to use a tool or operation outside the restricted set regardless of what the model requests.

#### Scenario: Plan mode excludes shell and MCP-sourced tools from the catalog
- **WHEN** a generation starts in plan mode
- **THEN** the tool catalog offered to the model SHALL NOT include the shell tool, the file-edit tool, or any MCP-sourced tool

#### Scenario: Plan mode narrows the file tool to read-only
- **WHEN** a generation starts in plan mode
- **THEN** the tool catalog offered to the model SHALL only allow the file tool's read operation, not its write operation

#### Scenario: Plan mode retains read-only search tools
- **WHEN** a generation starts in plan mode
- **THEN** the tool catalog offered to the model SHALL include the content-search and filename-search tools

#### Scenario: Plan mode retains read-only Skill tools
- **WHEN** a generation starts in plan mode
- **THEN** the tool catalog offered to the model SHALL include `list_skills`, `load_skill`, and `read_skill_resource`
- **AND** those tools SHALL remain unable to mutate Skill content, state, bindings, configuration, or resources

#### Scenario: Plan mode retains configured read-only LSP tools
- **WHEN** a generation starts in plan mode for a trusted local workspace with LSP available
- **THEN** the catalog SHALL include `find_definition`, `find_references`, `get_hover`, and `get_diagnostics`

#### Scenario: Plan mode still allows saving memories
- **WHEN** a generation starts in plan mode
- **THEN** the tool catalog offered to the model SHALL still include the remember tool

#### Scenario: Plan-mode OnePiece can inspect existing Artifacts
- **WHEN** a OnePiece generation starts in plan mode and Artifact/OCR readiness passes
- **THEN** the catalog MAY include bounded metadata/read operations for existing authorized Artifacts and OCR extraction that returns only its in-memory bounded result
- **AND** it SHALL exclude Artifact creation/publication and derived-Artifact output

#### Scenario: Plan mode excludes extended effectful tools
- **WHEN** a OnePiece generation starts in plan mode
- **THEN** the catalog SHALL exclude Browser navigation/interaction, Web search/fetch, `code_execution`, `delegate_cli`, `apply_delegation_changes`, Artifact publication, and any OCR operation that persists a derived Artifact

#### Scenario: A disallowed tool call is rejected even if requested
- **WHEN** the model requests the shell tool, the file-edit tool, an MCP-sourced tool, a file write operation, a mutating Skill or Artifact operation, an unadvertised mutating LSP or OCR operation, Browser/Web access, code execution, CLI delegation, or delegated-change application while the session is in plan mode
- **THEN** the system SHALL reject the call as an error outcome without executing it, regardless of whether the tool appeared in the offered catalog

#### Scenario: Other permission modes are unaffected
- **WHEN** a generation starts with a permission mode other than plan mode
- **THEN** the tool catalog and tool execution behavior SHALL remain governed by that mode's existing permission and tool-availability rules
- **AND** the read-only fixed Skill tools and eligible OnePiece-only tools SHALL be available according to their effective runtime predicates

#### Scenario: Plan mode offers the request to leave plan mode
- **WHEN** a generation starts in plan mode
- **THEN** the tool catalog offered to the model SHALL include the tool by which the model requests to leave plan mode
