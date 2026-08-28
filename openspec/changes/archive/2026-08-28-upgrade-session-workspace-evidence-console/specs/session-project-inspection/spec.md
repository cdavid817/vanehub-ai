## ADDED Requirements

### Requirement: Provider-neutral session workspace inspection

The workspaces application layer SHALL resolve one provider-neutral read-only inspection target from the registered session and SHALL use local, SSH, or simulated providers without exposing provider-specific filesystem or transport behavior to React.

#### Scenario: Inspect a local session

- **WHEN** a selected session is bound to a local project or worktree
- **THEN** the service SHALL use the local provider while preserving current canonical confinement, symlink, size, encoding, Git-locale, and diff bounds

#### Scenario: Inspect a remote session

- **WHEN** a selected session is bound to a current trusted SSH profile and remote workspace root
- **THEN** the service SHALL resolve the SSH provider through the workspaces service boundary
- **AND** workspaces SHALL consume remote channels only through the published `ssh_connections::api` contract

#### Scenario: Inspect a Web fixture

- **WHEN** the browser/Web runtime requests the same operation
- **THEN** the Web/mock adapter SHALL return deterministic contract-compatible inspection data labelled simulated
- **AND** it SHALL not claim native filesystem, SSH, Git, or process side effects

### Requirement: Explicit workspace inspection capabilities

Every session workspace inspection provider SHALL return typed capabilities for directory listing, bounded text reads, path search, content search, Git status, Git diff, and invalidation mode.

#### Scenario: Remote helper and Git are available

- **WHEN** the SSH provider verifies the current profile/host authority, supported remote helper, and Git executable
- **THEN** Files, Documents, Git status, and Git diff capabilities SHALL be available according to their independently verified prerequisites

#### Scenario: Remote helper is unavailable

- **WHEN** the remote host cannot run the supported bounded helper protocol
- **THEN** Files and Documents SHALL show a typed capability-unavailable state with safe remediation
- **AND** an otherwise valid remote Shell SHALL remain available

#### Scenario: Ripgrep is unavailable

- **WHEN** remote read/list is available but the verified content-search executable is not
- **THEN** read/list MAY remain available while content search reports its own typed unavailability
- **AND** the service SHALL not silently execute an unbounded recursive fallback

### Requirement: Remote workspace-root confinement

Remote inspection SHALL validate the registered remote root and every requested relative target on the remote host before reading or executing Git/search inspection.

#### Scenario: Read a valid remote relative path

- **WHEN** the remote helper resolves the registered root and requested target to a canonical target under that root
- **THEN** it SHALL perform the bounded inspection and return normalized relative metadata

#### Scenario: Reject remote traversal or symlink escape

- **WHEN** a relative path uses traversal, an absolute replacement, a NUL value, or a symbolic link resolving outside the registered remote root
- **THEN** the remote provider SHALL reject it before returning target metadata or content

#### Scenario: Remote profile revision becomes stale

- **WHEN** the bound SSH profile revision, endpoint, host key authority, or credential authority no longer matches the session binding
- **THEN** inspection SHALL stop with a typed stale-binding result
- **AND** it SHALL not retry through an older pooled authority

### Requirement: Stable directory continuation and targeted invalidation

File-tree providers SHALL return stable per-directory keyset continuation and SHALL invalidate affected queries when local watch, bounded remote polling, or execution evidence observes a relevant workspace change.

#### Scenario: Continue a large directory

- **WHEN** one directory contains more entries than a page bound
- **THEN** the provider SHALL return an opaque query-bound continuation cursor for that directory
- **AND** the Files tab SHALL be able to load the next entries without resetting unrelated expanded directories

#### Scenario: New file is inserted before continuation

- **WHEN** a file is created after the first directory page and before the next cursor is consumed
- **THEN** continuation SHALL remain stable according to the original keyset boundary
- **AND** a later invalidation/refresh SHALL expose the new entry without duplicating prior rows

#### Scenario: Agent changes one file

- **WHEN** execution evidence or a provider watcher reports a change to one relative path
- **THEN** the frontend SHALL invalidate the affected parent directory, preview/document, Git status/diff, and review queries
- **AND** it SHOULD preserve unrelated expanded nodes and selections

#### Scenario: Provider watch fails

- **WHEN** native watch or remote polling becomes unavailable
- **THEN** the provider SHALL degrade to explicit/event-derived refresh with an honest watch capability state
- **AND** it SHALL not claim the tree is live-updated

### Requirement: Workspace Quick Open and bounded content search

The Files workspace SHALL provide service-backed Quick Open path search and bounded content search with cancellation, stable pages, provider coverage, and result navigation.

#### Scenario: Quick Open a path

- **WHEN** the user enters a path query
- **THEN** the service SHALL return bounded deterministically ordered relative-path matches with file type and provider coverage
- **AND** selecting a result SHALL expand/select its tree path and open its preview when supported

#### Scenario: Search file content

- **WHEN** the user submits a non-empty content query
- **THEN** the provider SHALL return bounded matches with relative path, line, column, and bounded safe snippet
- **AND** selecting a match SHALL open the file and navigate to the matched line

#### Scenario: Cancel an in-progress search

- **WHEN** the user changes or cancels a potentially long local or remote search
- **THEN** the backend-managed operation SHALL stop according to its cancellation contract
- **AND** stale results SHALL not replace a newer query

#### Scenario: Search is partial

- **WHEN** provider limits, ignored paths, timeout, unavailable tool, or result bounds prevent a complete search
- **THEN** the response SHALL identify partial or unavailable coverage and safe reason codes

### Requirement: Enhanced bounded read-only file preview

The Files tab SHALL keep read-only file preview bounded while adding line navigation, in-preview search, syntax-aware presentation, metadata, and evidence actions.

#### Scenario: Preview a supported source file

- **WHEN** a supported text file within the existing content bound is selected
- **THEN** the UI SHALL display line numbers, safe syntax highlighting when a grammar is available, encoding/newline metadata, and bounded in-preview find

#### Scenario: Refresh selected file

- **WHEN** the selected file is refreshing or another file is loading
- **THEN** the previous successfully loaded preview SHALL remain visible with stale/refreshing status until the new request succeeds

#### Scenario: File changed before refresh completes

- **WHEN** the provider reports a different file witness during preview refresh
- **THEN** the UI SHALL present the current service result and identify that the prior view was stale
- **AND** it SHALL not expose editing or silently overwrite any file

#### Scenario: Preview unsupported content

- **WHEN** a file is binary, oversized, invalid text, or unsupported by the provider
- **THEN** the preview SHALL show bounded metadata and a typed unavailable state without loading unrestricted bytes

### Requirement: Read-only document workspace navigation

The Documents tab SHALL provide recent documents, path search, source/preview modes, and a bounded heading outline while preserving existing safe Markdown behavior and read-only semantics.

#### Scenario: Open a Markdown document

- **WHEN** a bounded Markdown document is selected
- **THEN** Documents SHALL support Source and Preview modes and a heading outline derived from the returned content
- **AND** Markdown links, images, code, math, and Mermaid SHALL continue through existing safe renderers

#### Scenario: Switch documents during loading

- **WHEN** another document is selected while the new content loads
- **THEN** the previous document SHALL remain visible with loading status until the new document succeeds
- **AND** a failure SHALL leave the document list and another selection usable

#### Scenario: Select an outline entry

- **WHEN** the user activates a heading in the outline
- **THEN** the document view SHALL navigate to the corresponding bounded heading anchor without scrolling the whole workspace shell

#### Scenario: Remote documents are unavailable

- **WHEN** the SSH provider cannot safely list/read documents
- **THEN** Documents SHALL show the typed provider capability reason
- **AND** it SHALL not present a generic empty document list as proof that no documents exist

### Requirement: File and document execution-evidence links

Files and Documents SHALL expose safe links to correlated runs, commands, changes, reviews, and findings when the execution evidence service reports them.

#### Scenario: Open executions related to a file

- **WHEN** a selected relative path has one or more retained safe file-mutation observations
- **THEN** Files SHALL expose a bounded related-execution action
- **AND** selecting a record SHALL navigate through the shared workspace evidence scope

#### Scenario: Open Changes for a modified document

- **WHEN** a selected document is present in current structured Git status or review data
- **THEN** Documents SHALL offer an action to open the corresponding Changes/Review file

#### Scenario: No retained evidence exists

- **WHEN** a file has no retained or available evidence links
- **THEN** the UI SHALL omit or disable those links with an honest unavailable state
- **AND** it SHALL not infer an Agent or command solely from file timestamps

### Requirement: Provider-backed inspection service boundary

React SHALL obtain local, remote, and simulated Files, Documents, search, Git status, and Git diff data through one frontend service contract with Tauri and Web/mock adapters.

#### Scenario: Desktop invokes remote inspection

- **WHEN** React requests remote directory, file, search, status, or diff data
- **THEN** it SHALL call the frontend workspace-inspection service
- **AND** the Tauri adapter SHALL invoke declared workspaces commands rather than React constructing SSH commands or reading remote paths directly

#### Scenario: Provider operation is long-running

- **WHEN** a remote scan, search, or Git inspection may exceed an immediate command boundary
- **THEN** it SHALL use backend-managed operation state with a stable operation id, progress or running status, cancellation when supported, terminal result/error, and unified redacted diagnostics
