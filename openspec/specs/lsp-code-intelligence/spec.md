# lsp-code-intelligence Specification

## Purpose
Defines how native API Agents obtain bounded, workspace-scoped live definitions, references, hover information, and diagnostics from configured language servers without confusing failures with valid empty results.
## Requirements
### Requirement: Read-only LSP tools are conditionally available
The system SHALL expose `find_definition`, `find_references`, `get_hover`, and `get_diagnostics` to a native API Agent only when the current session has a trusted local workspace and a matching language-server configuration is enabled and discoverable. Tool availability SHALL NOT require persistent Tree-sitter code indexing or an already-running server.

#### Scenario: Trusted Rust workspace starts with no process
- **WHEN** a generation begins for a trusted local Rust workspace with an available configured server but no running process
- **THEN** the four read-only LSP tools SHALL be offered
- **AND** the first applicable call MAY start the server on demand

#### Scenario: Code indexing is disabled
- **WHEN** LSP is enabled and trusted for a local workspace whose code index is disabled or absent
- **THEN** supported LSP tools SHALL remain available

#### Scenario: Session is remote
- **WHEN** the current session resolves to an SSH or other remote workspace
- **THEN** the foundation SHALL NOT offer or execute local LSP tools for that session

### Requirement: Semantic queries are implicitly workspace scoped
Every model-visible LSP tool SHALL derive its workspace from the current session, accept only normalized relative file paths, and filter all returned locations to `file:` URIs inside that canonical workspace. Model-supplied workspace ids, roots, server paths, or URI schemes SHALL NOT select another scope.

#### Scenario: Definition points outside the workspace
- **WHEN** a server returns a definition location outside the current canonical workspace
- **THEN** that location SHALL NOT be returned to the Agent
- **AND** the result SHALL report a bounded filtered-location count

#### Scenario: Model attempts to select another workspace
- **WHEN** a tool input includes an unsupported workspace id, absolute root, or server selector
- **THEN** the system SHALL ignore or reject that field according to the declared schema
- **AND** it SHALL continue to enforce the current session workspace

### Requirement: Agent and protocol positions are normalized explicitly
Position-taking tools SHALL accept 1-based line and column values, convert them using the negotiated LSP position encoding, and return 1-based file ranges with explicit exclusive end positions. Invalid or out-of-range positions SHALL fail before a protocol request is sent.

#### Scenario: UTF-16 server receives a non-ASCII position
- **WHEN** a requested line contains characters whose UTF-8 byte width differs from UTF-16 units
- **THEN** the client SHALL convert the 1-based Agent column to the correct UTF-16 LSP character offset

#### Scenario: Input line is outside the document
- **WHEN** a tool input names a line beyond the current bounded disk snapshot
- **THEN** the tool SHALL return an invalid-position status without sending an LSP request

### Requirement: Definition and reference results are normalized and bounded
`find_definition` SHALL normalize null, single, multiple, and linked definition results to workspace-relative ranges with bounded current-disk previews. `find_references` SHALL normalize and sort workspace-relative reference ranges, cap returned definitions at 20 and references at 50, and preserve total and truncation metadata.

#### Scenario: Server returns linked definitions
- **WHEN** a definition response contains one or more location links
- **THEN** the tool SHALL normalize their target ranges into the common definition result shape

#### Scenario: References exceed the return cap
- **WHEN** a references response contains more than 50 accepted workspace locations
- **THEN** the tool SHALL return at most 50 deterministically ordered references
- **AND** it SHALL report the accepted total and `truncated: true`

### Requirement: Hover output preserves useful semantics within hard limits
`get_hover` SHALL return a normalized optional signature, bounded documentation, and normalized range when available. Markup kinds and server-specific hover shapes SHALL be converted without returning unbounded source or documentation content.

#### Scenario: Hover contains signature and Markdown documentation
- **WHEN** a server returns marked signature and Markdown documentation for the requested symbol
- **THEN** the tool SHALL return normalized bounded signature and documentation fields
- **AND** it SHALL preserve no executable HTML behavior

#### Scenario: Symbol has no hover
- **WHEN** a ready server returns no hover result
- **THEN** the tool SHALL return `status: ready` with no hover value rather than an unavailable or failed status

### Requirement: Diagnostics are version-aware snapshots
The system SHALL cache bounded `textDocument/publishDiagnostics` snapshots per workspace document and SHALL expose severity, message, source, code, and range only for the current workspace. `get_diagnostics` SHALL distinguish a current empty snapshot, a stale snapshot, waiting for first publication, timeout, and unavailable server state.

#### Scenario: Current document has no diagnostics
- **WHEN** the server publishes an empty diagnostic list matching the current document version
- **THEN** `get_diagnostics` SHALL return `status: ready`, `stale: false`, and an empty diagnostics array

#### Scenario: Diagnostics belong to an older version
- **WHEN** the disk document version advances after the last published diagnostics
- **THEN** `get_diagnostics` SHALL mark the prior snapshot stale and wait only within its bounded deadline for a newer publication

#### Scenario: Diagnostic contains related outside location
- **WHEN** a diagnostic contains related information for a URI outside the canonical workspace
- **THEN** the outside related location SHALL be omitted from the Agent result

### Requirement: Tool outcomes distinguish degradation from no result
Every LSP tool result SHALL include a status from `ready`, `warming`, `timeout`, `unavailable`, or `failed`, plus available server/language identity, document version, stale state, returned count, total, and truncation metadata. Optional LSP failure SHALL be returned as a bounded tool outcome and SHALL NOT terminate the Agent generation.

#### Scenario: Server is still warming
- **WHEN** a tool call cannot run because its server is starting or initializing
- **THEN** the tool SHALL return `status: warming`
- **AND** it SHALL NOT return a misleading ready empty result

#### Scenario: Optional LSP request fails
- **WHEN** an LSP request fails after the Agent tool call has begun
- **THEN** the Agent SHALL receive a bounded failed or unavailable tool result
- **AND** the provider tool loop SHALL remain able to continue

### Requirement: Web runtime provides deterministic contract parity
The Web/mock runtime SHALL implement the same LSP configuration, trust, discovery, test, status, and tool-result contract without reading the host filesystem, launching a process, or contacting a language server.

#### Scenario: Web mode tests a server
- **WHEN** the frontend requests an LSP server test through the Web adapter
- **THEN** it SHALL return a deterministic mock phase result with the desktop contract shape
- **AND** it SHALL perform no native process or filesystem operation

#### Scenario: Web mode simulates a semantic tool result
- **WHEN** the Web/mock Agent runtime requests any of the four read-only LSP tool-result shapes
- **THEN** it SHALL return a deterministic `unavailable` envelope with the same normalized metadata and tool-specific payload key as the native Agent result
- **AND** it SHALL not inspect the requested path or perform filesystem, process, or network access
