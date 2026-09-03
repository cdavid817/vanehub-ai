# session-project-inspection Specification

## Purpose
Defines confined, bounded, read-only inspection of session project files, documents, Git status, and structured diffs.
## Requirements
### Requirement: Session-root access confinement
Project inspection operations SHALL resolve their root from the registered session and MUST reject access outside the canonical session root.

#### Scenario: Read a valid relative path
- **WHEN** a request contains a relative path whose canonical target remains under the session root
- **THEN** the native runtime SHALL evaluate the bounded inspection operation for that target

#### Scenario: Reject parent traversal
- **WHEN** a relative path attempts parent traversal outside the session root
- **THEN** the native runtime SHALL reject the request with a concise validation error

#### Scenario: Reject symlink escape
- **WHEN** a path resolves through a symbolic link to a target outside the session root
- **THEN** the native runtime SHALL reject the request and SHALL NOT return target metadata or content

#### Scenario: Resolve missing session root
- **WHEN** the selected session has no available project or working folder
- **THEN** the service SHALL return a typed unavailable result rather than inspecting an arbitrary process directory

### Requirement: Lazy project file tree
The Files tab SHALL load immediate directory children on demand with deterministic sorting and bounded results.

#### Scenario: Expand a directory
- **WHEN** the user expands a directory node
- **THEN** the service SHALL return its non-hidden immediate children with directories before files and names sorted deterministically

#### Scenario: Reach directory entry limit
- **WHEN** a directory contains more entries than the configured bound
- **THEN** the response SHALL mark the result as truncated and the UI SHALL display a localized partial-result state

#### Scenario: Collapse and reopen a directory
- **WHEN** the user collapses and reopens a previously loaded directory in the same mounted tab
- **THEN** the tree SHALL retain its expanded-node and selection state unless its query was invalidated

### Requirement: Bounded read-only file preview
The Files and Documents tabs SHALL preview supported text files without exposing binary or oversized content.

#### Scenario: Read a supported text file
- **WHEN** the selected file is text and no larger than 1 MiB
- **THEN** the service SHALL return decoded content and metadata for read-only display

#### Scenario: Reject oversized content
- **WHEN** the selected file exceeds 1 MiB
- **THEN** the service SHALL return an oversized marker without loading the full content into the frontend

#### Scenario: Detect binary content
- **WHEN** the selected file is detected as binary
- **THEN** the service SHALL return a binary marker and SHALL NOT return its raw bytes as text

#### Scenario: File changes during read
- **WHEN** a file disappears or becomes inaccessible before content is read
- **THEN** the UI SHALL show a concise localized error and remain usable for another selection

### Requirement: Structured Git status
The Changes tab SHALL expose structured index and worktree status for the selected session root when it is a Git repository.

#### Scenario: Show changed paths
- **WHEN** Git reports modified, added, deleted, renamed, conflicted, or untracked paths
- **THEN** the service SHALL preserve index and worktree status separately for each path

#### Scenario: Display changed-path status
- **WHEN** Changes renders a structured Git status entry
- **THEN** the UI SHALL show conventional index/worktree codes and localized status labels that distinguish unmodified, modified, added, deleted, renamed, copied, conflicted, and untracked states

#### Scenario: Non-Git session
- **WHEN** the selected session root is not a Git repository
- **THEN** Changes SHALL show a localized non-Git empty state rather than a raw command failure

#### Scenario: Git command fails
- **WHEN** Git inspection fails for another reason
- **THEN** the native runtime SHALL persist redacted diagnostics through unified logging and return a concise error through the service boundary

### Requirement: Unified and split Git diff views
The Changes tab SHALL render working-tree and staged diffs from one structured file/hunk/line model in unified or split view.

#### Scenario: Select diff source
- **WHEN** a path has staged and working-tree changes
- **THEN** the user SHALL be able to inspect each source without combining their hunks ambiguously

#### Scenario: Switch diff view
- **WHEN** the user switches between unified and split view
- **THEN** the viewer SHALL reuse the same structured diff and preserve the selected file and scroll context where practical

#### Scenario: View untracked text file
- **WHEN** an untracked text file is within the content bound
- **THEN** the diff SHALL represent its lines as additions against an empty file

#### Scenario: View binary or oversized diff
- **WHEN** a changed file is binary or exceeds the configured diff bound
- **THEN** the viewer SHALL show status metadata without attempting to render textual hunks

#### Scenario: Parse renamed file
- **WHEN** Git reports a rename
- **THEN** the structured diff SHALL preserve old and new paths for display

### Requirement: Witnessed review snapshots and guarded Git mutation
The workspace service SHALL extend its confined structured Git inspection with stable review file/hunk fingerprints and explicitly guarded whole-file and hunk revert operations that validate the owning session root and current witnesses immediately before mutation.

#### Scenario: Create review snapshot
- **WHEN** a review requests the session's changed files
- **THEN** `workspaces` SHALL produce bounded structured metadata and deterministic content witnesses without persisting full diff content

#### Scenario: Apply reverse hunk
- **WHEN** a confirmed reverse-hunk request has a current witness and an exact patch target
- **THEN** `workspaces` SHALL apply only that patch under its mutation guard and return the resulting witness

#### Scenario: Refuse unsafe mutation
- **WHEN** path confinement, file type, size, fingerprint, or exact patch application validation fails
- **THEN** `workspaces` SHALL fail closed without partial writes

### Requirement: Git inspection outcomes are locale-independent

The native runtime SHALL execute Git inspection commands with a pinned message locale so that outcome classification — non-Git detection and untracked-path detection in particular — does not depend on the host system's display language. User-facing presentation of the classified outcome SHALL remain localized as specified elsewhere in this capability.

#### Scenario: Non-Git directory on a non-English host

- **WHEN** the selected session root is not a Git repository and the host locale is not English
- **THEN** the runtime SHALL classify it as the non-Git case
- **AND** Changes SHALL show the localized non-Git empty state rather than a raw command failure

#### Scenario: Untracked path on a non-English host

- **WHEN** an untracked path is probed on a host whose locale is not English
- **THEN** the runtime SHALL classify it as untracked rather than as a Git command failure

#### Scenario: Caller-supplied environment still applies

- **WHEN** a Git invocation is made with explicit caller-supplied environment variables
- **THEN** those variables SHALL take precedence over the pinned locale default for that invocation

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

### Requirement: Workspace inspection cancellation is generation-safe and RAII-owned

Each cancellable workspace inspection SHALL register a unique generation under its search id and SHALL own that registration through an RAII-equivalent async guard. Superseding a search SHALL cancel the previous generation, and completion, failure, abort, or drop of one generation MUST NOT remove or mutate a newer generation.

#### Scenario: Older search finishes after its replacement starts

- **WHEN** search A is registered, search B starts with the same search id and supersedes A, and A finishes afterward
- **THEN** A SHALL remove only its own generation if still present
- **AND** B SHALL remain registered and cancellable
- **AND** A's late result SHALL NOT overwrite B's current frontend state

#### Scenario: Owning future is aborted

- **WHEN** the async future that owns a running inspection is aborted or dropped before normal completion
- **THEN** its registration guard SHALL signal the worker cancellation token
- **AND** the worker SHALL stop at a bounded checkpoint
- **AND** registry/admission cleanup SHALL occur after that worker exits without requiring an explicit finish call

### Requirement: Inspection budgets account for actual work and report completeness

Every recursive or potentially large workspace inspection SHALL enforce finite budgets for directories, entries, files, bytes, metadata/canonicalization work, retained candidates, results, depth, and a monotonic deadline as applicable. Budget accounting SHALL include ignored, unreadable, non-matching, and rejected entries that still consumed work.

Results SHALL distinguish `Complete`, `Partial`, and `Unavailable` and SHALL include a stable primary reason code whenever coverage is not complete.

#### Scenario: Many non-matching entries exhaust work budget

- **WHEN** a traversal visits the configured entry limit without producing a match
- **THEN** it SHALL stop before visiting another entry
- **AND** it SHALL return an empty `Partial` result with `entry_budget_exhausted`
- **AND** the UI SHALL NOT present that result as definitive proof that no match exists

#### Scenario: User cancels during a file read

- **WHEN** cancellation is signalled while content search is reading a large eligible file
- **THEN** the worker SHALL observe cancellation at a bounded chunk checkpoint
- **AND** it SHALL return or terminate with `cancelled` or `superseded` coverage for that generation

#### Scenario: Traversal completes under policy and budget

- **WHEN** every eligible entry under the effective recursive ignore policy is exhausted without omission, failure, cancellation, or budget stop
- **THEN** coverage SHALL be `Complete`
- **AND** an empty result MAY be presented as no matches for that request

### Requirement: Recursive content and document inspection use streaming bounded memory

Content search and recursive document discovery SHALL process eligible entries as a bounded stream and MUST NOT first materialize the complete candidate-file set. Their retained memory SHALL be limited to traversal state, current bounded file/chunk state, bounded results/snippets, and bounded coverage/error counters.

#### Scenario: Workspace contains more candidates than the candidate budget

- **WHEN** a workspace contains a very large number of eligible files
- **THEN** content search SHALL open/process candidates incrementally
- **AND** instrumentation SHALL show no full candidate vector proportional to workspace size
- **AND** cancellation, deadline, and byte/file budgets SHALL remain enforceable during traversal

#### Scenario: Default document discovery encounters dependency output

- **WHEN** recursive document discovery reaches `node_modules`, `target`, `dist`, or another configured generated/dependency directory without an explicit include override
- **THEN** the shared recursive ignore policy SHALL skip descent
- **AND** the skipped subtree SHALL not consume child-entry/file-read work beyond the directory entry required to apply the policy

#### Scenario: User explicitly navigates to an ignored directory

- **WHEN** a user directly lists or reads a path that is ignored only by recursive discovery policy
- **THEN** the system SHALL continue to allow that explicit operation subject to existing root, authorization, type, and size rules
- **AND** it SHALL NOT treat ignore policy as an access-control denial

### Requirement: Path and directory pagination retain only bounded selections

Path search and immediate directory pagination SHALL use a bounded selection algorithm whose retained candidate/page memory is proportional to configured result/page limits. They MUST NOT retain and sort every eligible entry solely to return one bounded page.

#### Scenario: Directory contains hundreds of thousands of entries

- **WHEN** the user requests a page of `N` entries from a very large immediate directory
- **THEN** the implementation SHALL retain at most `N + 1` selected entries plus fixed traversal overhead
- **AND** all scanned entries SHALL still consume entry/metadata budgets
- **AND** incomplete scanning SHALL be reported separately from normal `has_more` pagination

#### Scenario: Directory changed between pages

- **WHEN** a versioned cursor's detectable directory fingerprint, directory identity, order mode, or policy identity no longer matches the next request
- **THEN** the adapter SHALL return `invalid_cursor` or `stale_cursor`
- **AND** the frontend SHALL restart and replace pagination rather than append an incompatible page

### Requirement: Workspace inspection adapters preserve admission and coverage parity

Local, remote, Tauri, and Web/mock implementations SHALL expose the same generation, coverage state, reason-code, cursor, and budget-summary semantics. Global and per-workspace admission SHALL be acquired before blocking or remote work starts and SHALL remain held until that work actually exits.

#### Scenario: Per-workspace inspection capacity is exhausted

- **WHEN** another independent inspection cannot obtain a per-workspace permit within the finite admission policy
- **THEN** it SHALL return `Unavailable` with `inspection_busy`
- **AND** it SHALL NOT enqueue an unbounded blocking task or launch a remote provider process

#### Scenario: Web mock hits a byte budget

- **WHEN** deterministic Web/mock fixtures consume their configured simulated byte budget
- **THEN** Web/mock SHALL return the same Partial/byte-budget reason contract as native adapters
- **AND** it SHALL clearly remain simulated and SHALL NOT claim a native filesystem scan occurred

