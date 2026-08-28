# lsp-code-intelligence Specification

## Purpose
Defines how native API Agents obtain bounded, workspace-scoped live definitions, references, hover information, and diagnostics from configured language servers without confusing failures with valid empty results.
## Requirements
### Requirement: Read-only LSP tools are conditionally available
The system SHALL expose its read-only LSP tools — `find_definition`, `find_references`, `get_hover`, `get_diagnostics`, `find_workspace_symbols`, `get_document_symbols`, `find_type_definition`, `find_implementations`, and `find_call_hierarchy` — to a native API Agent only when the current session has a trusted local workspace and a matching language-server configuration is enabled and discoverable. A tool whose method the configured server does not advertise SHALL still be offered and SHALL return an unavailable status when called, because support is a per-server fact discovered at initialize rather than a property of the session. Tool availability SHALL NOT require persistent Tree-sitter code indexing or an already-running server.

#### Scenario: Trusted Rust workspace starts with no process
- **WHEN** a generation begins for a trusted local Rust workspace with an available configured server but no running process
- **THEN** the read-only LSP tools SHALL be offered
- **AND** the first applicable call MAY start the server on demand

#### Scenario: Code indexing is disabled
- **WHEN** LSP is enabled and trusted for a local workspace whose code index is disabled or absent
- **THEN** supported LSP tools SHALL remain available

#### Scenario: Session is remote
- **WHEN** the current session resolves to an SSH or other remote workspace
- **THEN** the foundation SHALL NOT offer or execute local LSP tools for that session

#### Scenario: A server advertises only some of the methods
- **WHEN** a trusted workspace's server advertises definitions but not call hierarchy
- **THEN** both tools SHALL be offered to the Agent
- **AND** a call-hierarchy call SHALL return an unavailable status rather than the tool being withheld

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

### Requirement: LSP provides optional Context Engine candidates
Trusted and ready LSP definitions, references, and call relations SHALL be normalizable as bounded Context Engine candidates with server, language, document-version, range, truncation, and stale-state provenance. Call relations SHALL be an implemented source rather than a conditional one, and SHALL degrade exactly as definitions and references do when a server does not advertise the method.

#### Scenario: LSP returns definition and references
- **WHEN** a planned symbol query completes within existing LSP bounds
- **THEN** normalized locations SHALL enter the Context Engine candidate pipeline rather than append provider text directly

#### Scenario: LSP cannot serve the request
- **WHEN** trust, capability, readiness, timeout, cancellation, or server failure prevents a query
- **THEN** the source SHALL return its existing bounded degradation state
- **AND** it SHALL NOT fail candidate collection or generation

#### Scenario: LSP returns call relations
- **WHEN** a planned call-relation query completes within existing LSP bounds
- **THEN** the normalized relations SHALL enter the candidate pipeline with the same provenance definitions and references carry

#### Scenario: The server does not advertise call hierarchy
- **WHEN** a planned call-relation query targets a server whose negotiated record reports no call hierarchy support
- **THEN** the source SHALL return its bounded unavailable state
- **AND** definition, reference, retrieval, and Tree-sitter candidates SHALL remain eligible

### Requirement: Code intelligence performance evidence uses versioned repositories
Code intelligence benchmarks SHALL cover Tree-sitter incremental updates, workspace indexing, indexed search, and bounded LSP definition and reference queries using versioned small, medium, and large synthetic repository datasets.

#### Scenario: Code intelligence datasets are measured
- **WHEN** supported Tree-sitter, index, search, and LSP workloads run
- **THEN** results SHALL include bounded files, bytes, symbols, locations, queue work, and response items
- **AND** dedicated results SHALL include P50/P95 latency with commit, platform, profile, and dataset provenance

#### Scenario: LSP is unavailable or returns too many locations
- **WHEN** the server is unavailable, warming, timed out, or returns more than the supported cap
- **THEN** the benchmark SHALL preserve existing degradation semantics and deterministic response bounds without converting unavailable state into a false latency success

### Requirement: Workspace symbol search is bounded and workspace scoped
`find_workspace_symbols` SHALL accept a bounded query string, return matches as workspace-relative ranges with symbol name and kind, cap returned symbols at 50, and preserve accepted total and truncation metadata. Matches outside the canonical workspace SHALL be filtered before the Agent sees them, and the filtered count SHALL be reported.

#### Scenario: A query matches more symbols than the cap
- **WHEN** a workspace symbol response contains more than 50 accepted workspace matches
- **THEN** the tool SHALL return at most 50 deterministically ordered symbols
- **AND** it SHALL report the accepted total and `truncated: true`

#### Scenario: A match resolves outside the workspace
- **WHEN** a workspace symbol response contains a location outside the canonical workspace
- **THEN** that match SHALL NOT be returned
- **AND** the result SHALL report a bounded filtered-location count

#### Scenario: The query is empty
- **WHEN** the query string is empty or consists only of whitespace
- **THEN** the tool SHALL return an invalid-input status without sending a protocol request

#### Scenario: No symbol matches
- **WHEN** a ready server returns no workspace symbol matches
- **THEN** the tool SHALL return `status: ready` with an empty list rather than an unavailable status

### Requirement: Document symbols are returned as a bounded flattened outline
`get_document_symbols` SHALL return the symbols of one admitted workspace document as workspace-relative ranges with name, kind, and the name of the enclosing symbol where the server reports nesting. Nesting SHALL be flattened rather than returned as a tree, bounded by a declared depth, and the result SHALL cap returned symbols and preserve total and truncation metadata.

#### Scenario: The server returns a nested symbol tree
- **WHEN** a document symbol response nests symbols within symbols
- **THEN** the tool SHALL flatten them to a bounded depth, each carrying the name of its enclosing symbol
- **AND** symbols deeper than the declared bound SHALL be omitted and counted as truncated

#### Scenario: The server returns a flat symbol list
- **WHEN** a document symbol response contains no nesting
- **THEN** the tool SHALL return the symbols with no enclosing-symbol name
- **AND** the result SHALL be shaped identically to a flattened nested response

#### Scenario: The document is not admitted
- **WHEN** the requested document is outside the canonical workspace, hidden, binary, oversized, non-file, or reached through an escaping symbolic link
- **THEN** the tool SHALL reject it without sending its content to a language server

### Requirement: Type definition and implementation reuse the definition result shape
`find_type_definition` and `find_implementations` SHALL normalize null, single, multiple, and linked results into the same workspace-relative shape `find_definition` produces, with the same bounded current-disk previews, the same cap of 20, and the same total and truncation metadata.

#### Scenario: A type definition response contains location links
- **WHEN** a type definition response contains one or more location links
- **THEN** the tool SHALL normalize their target ranges into the common definition result shape

#### Scenario: An implementation query finds nothing
- **WHEN** a ready server returns no implementations for the requested position
- **THEN** the tool SHALL return `status: ready` with an empty list rather than an unavailable status

#### Scenario: The server does not advertise the method
- **WHEN** a configured server's negotiated record reports no type definition or implementation support
- **THEN** the corresponding query SHALL return an unavailable status without sending an unsupported request

### Requirement: Call hierarchy is a bounded three-step query
`find_call_hierarchy` SHALL prepare a call hierarchy item for a requested position, then request incoming or outgoing calls for it as the caller selected, and return workspace-relative ranges with the calling or called symbol name. The whole three-step exchange SHALL complete within one bounded deadline rather than each step carrying the single-request budget, SHALL cap returned relations, and SHALL preserve total and truncation metadata.

#### Scenario: Preparation resolves no item
- **WHEN** the prepare step returns no call hierarchy item for the requested position
- **THEN** the tool SHALL return `status: ready` with an empty list
- **AND** it SHALL NOT send a calls request

#### Scenario: The exchange exceeds its deadline
- **WHEN** the prepare and calls steps together exceed the bounded deadline
- **THEN** the tool SHALL return a timeout status
- **AND** no step SHALL remain in the pending-request map

#### Scenario: Preparation resolves several items
- **WHEN** the prepare step returns more than one call hierarchy item
- **THEN** the tool SHALL use the first deterministically ordered item
- **AND** it SHALL report that further items were not followed

#### Scenario: A relation resolves outside the workspace
- **WHEN** an incoming or outgoing call resolves to a location outside the canonical workspace
- **THEN** that relation SHALL NOT be returned
- **AND** the result SHALL report a bounded filtered-location count
