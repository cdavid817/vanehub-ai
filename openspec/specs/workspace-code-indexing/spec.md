# workspace-code-indexing Specification

## Purpose
TBD - created by archiving change workspace-code-indexing-foundation. Update Purpose after archive.
## Requirements
### Requirement: Code indexing is configured per workspace
The system SHALL assign each configured local workspace a stable opaque id and SHALL persist independent enablement, selected relative roots, enabled languages, exclusion patterns, and maximum file size for that workspace. Indexing SHALL be disabled by default.

#### Scenario: Enable one of two workspaces
- **WHEN** a user enables code indexing for one configured workspace
- **THEN** the system SHALL index only that workspace
- **AND** the other workspace SHALL remain disabled

#### Scenario: Restrict indexing to selected roots
- **WHEN** a user configures selected relative roots within a workspace
- **THEN** the system SHALL admit files only beneath those roots
- **AND** it SHALL reject roots that escape the canonical workspace boundary

#### Scenario: Configured root is unavailable
- **WHEN** a configured workspace root no longer exists
- **THEN** the system SHALL mark the workspace index unavailable
- **AND** it SHALL retain the workspace id, configuration, and existing index data

### Requirement: Runtime language selection controls parser admission
The system SHALL support JavaScript, TypeScript and TSX, Python, Rust, Go, Java, C, and C++ parser families and SHALL parse only languages enabled for the workspace.

#### Scenario: Disabled language file is discovered
- **WHEN** inventory discovers a source file whose language parser is disabled for that workspace
- **THEN** the system SHALL skip the file before reading or parsing its content
- **AND** it SHALL record a safe language-disabled reason count

#### Scenario: Language selection changes
- **WHEN** a user disables a previously enabled language
- **THEN** the system SHALL remove that language's file manifests, chunks, symbols, and vectors from the workspace index

### Requirement: File admission combines ignore rules and mandatory safety filters
The system SHALL respect nested `.gitignore` rules, validated user exclusion globs, enabled languages, binary detection, selected roots, and the configured file-size ceiling before parsing. Mandatory sensitive-file patterns and workspace boundary checks SHALL take precedence over user configuration.

#### Scenario: User exclusion matches generated code
- **WHEN** an admitted path matches a configured pattern such as `*.generated.ts` or `vendor/**`
- **THEN** the system SHALL skip the file before parsing or embedding

#### Scenario: File exceeds the size ceiling
- **WHEN** file metadata reports a size greater than the workspace limit
- **THEN** the system SHALL not read, parse, persist, or embed the file content
- **AND** it SHALL count the file under a safe size-limit reason

#### Scenario: Invalid exclusion pattern is saved
- **WHEN** a user attempts to save an invalid exclusion glob
- **THEN** the system SHALL reject the configuration update
- **AND** it SHALL preserve the last valid configuration

#### Scenario: Symlink escapes the workspace
- **WHEN** a discovered path resolves outside the canonical workspace boundary
- **THEN** the system SHALL reject the path without reading its target

### Requirement: Sensitive files never enter the index
The system SHALL maintain a non-overridable, case-normalized denylist for environment files, credential files, private keys, certificates, and common credential directories. Matching files SHALL NOT be read for indexing, persisted in retrieval storage, or sent to an embedding provider.

#### Scenario: Environment file matches mandatory denylist
- **WHEN** inventory discovers `.env`, `.env.local`, or an equivalent denied environment file
- **THEN** the system SHALL skip it before content access
- **AND** no index or embedding row SHALL contain its content

#### Scenario: User pattern attempts to re-include a key
- **WHEN** user configuration would otherwise include a file matching `*.key`, `*.pem`, or another mandatory pattern
- **THEN** the mandatory denylist SHALL win

### Requirement: Code secrets are redacted before persistence and embedding
The system SHALL apply the unified sensitive-information policy to admitted code before searchable chunk text is persisted, embedded, logged, audited, or returned from `search_code`. Raw code content SHALL NOT be duplicated into retrieval storage.

#### Scenario: Source contains a hard-coded token
- **WHEN** an admitted source file contains a detected credential assignment or token
- **THEN** the persisted and embedded chunk SHALL contain a redacted marker instead of the sensitive value
- **AND** the file manifest MAY retain only a one-way content hash for change detection

#### Scenario: Redacted chunk is returned
- **WHEN** code search matches a chunk containing a redacted value
- **THEN** the returned snippet SHALL remain redacted
- **AND** the result SHALL still include its file and line location

### Requirement: Tree-sitter produces bounded typed chunks and symbols
The system SHALL parse admitted code with the selected Tree-sitter grammar and SHALL persist bounded chunks with workspace id, normalized relative path, language, line range, symbol name, symbol kind, chunk key, and index version. It SHALL persist symbol definition metadata during the same file transaction.

#### Scenario: Function definition is indexed
- **WHEN** Tree-sitter identifies a function definition in an admitted file
- **THEN** at least one chunk SHALL carry that function's name, kind, and definition range

#### Scenario: Symbol exceeds the embedding budget
- **WHEN** one symbol is larger than the configured chunk budget
- **THEN** the system SHALL split it on named syntax nodes or bounded line windows
- **AND** every resulting chunk SHALL remain attributable to the source symbol and file range

#### Scenario: File contains recoverable syntax errors
- **WHEN** Tree-sitter can parse valid named subtrees around syntax errors
- **THEN** the system SHALL index only bounded chunks derived from valid subtrees
- **AND** it SHALL NOT embed an unparsed raw-file fallback

### Requirement: File manifests drive selective reconciliation
The system SHALL persist a per-workspace file manifest and SHALL read, hash, parse, and replace only files that are new, explicitly changed, or whose manifest fingerprint changed. It SHALL support targeted reconciliation of created, modified, renamed, and deleted relative paths.

#### Scenario: Unchanged inventory entry is reconciled
- **WHEN** a periodic inventory finds a file whose manifest fingerprint and index version are unchanged
- **THEN** the system SHALL not read, hash, parse, or re-embed that file

#### Scenario: Metadata changes but content does not
- **WHEN** a targeted file produces the same raw content hash as its manifest
- **THEN** the system SHALL update safe metadata as needed
- **AND** it SHALL not replace chunks or requeue embeddings

#### Scenario: Indexed file is deleted
- **WHEN** targeted reconciliation or inventory confirms an indexed file no longer exists
- **THEN** the system SHALL transactionally remove its manifest, chunks, symbols, FTS entries, and vectors

#### Scenario: File is renamed
- **WHEN** reconciliation receives a rename from one relative path to another
- **THEN** the system SHALL remove the old path's index data and index the new path subject to current admission rules

### Requirement: Index versions make parser changes observable
The system SHALL persist a code index version covering grammar compatibility, Tree-sitter queries, chunking, and redaction policy. A version mismatch SHALL mark affected workspace files stale and SHALL rebuild them in bounded batches.

#### Scenario: Grammar compatibility version changes
- **WHEN** the runtime code index version differs from a workspace file manifest
- **THEN** the system SHALL exclude stale vectors from code vector search
- **AND** it SHALL queue that file for rebuilding without affecting agent-memory rows

### Requirement: External embedding requires cost confirmation and throttling
The system SHALL complete safe local parsing and FTS indexing before external embedding, SHALL display the exact chunk input count and estimated batch request count, and SHALL require workspace-specific user confirmation before the first external embedding run. Embedding requests SHALL use bounded cross-batch throttling and hard timeouts.

#### Scenario: Initial local index is ready
- **WHEN** local parsing finishes for an externally configured embedding provider
- **THEN** the workspace SHALL enter `awaiting_embedding_confirmation`
- **AND** no code chunk SHALL be sent externally before confirmation

#### Scenario: User confirms embedding
- **WHEN** the user confirms the displayed provider, model, input count, and estimated requests
- **THEN** the system SHALL enqueue that workspace's redacted chunks for embedding

#### Scenario: Provider returns Retry-After
- **WHEN** the provider rate-limits an embedding batch with a bounded `Retry-After` value
- **THEN** the worker SHALL wait at least that interval before the next request for that provider profile
- **AND** it SHALL remain cancellable between batches

### Requirement: Code indexing can be cooperatively cancelled
The system SHALL stop claiming new file and embedding work when a workspace is disabled, deleted, or the application shuts down. Results from an in-flight operation SHALL be discarded when its workspace generation is stale.

#### Scenario: Disable during embedding
- **WHEN** a user disables a workspace while one embedding request is in flight
- **THEN** the system SHALL claim no further batches for that workspace
- **AND** it SHALL not store the in-flight response after the workspace generation becomes stale

### Requirement: Code search is implicitly scoped and structurally typed
The system SHALL expose a `search_code` tool only for an enabled local current-session workspace. Its model-visible input SHALL contain exactly `query` and `limit`, and every candidate path SHALL be restricted to the resolved workspace id before vector deserialization or FTS ranking.

#### Scenario: Search returns a code hit
- **WHEN** `search_code` finds a matching chunk in the current workspace
- **THEN** it SHALL return a redacted snippet, normalized relative file path, start and end lines, language, optional symbol name and kind, and matched-via value

#### Scenario: Another workspace has a stronger match
- **WHEN** a different indexed workspace contains a higher-scoring match
- **THEN** that match SHALL NOT be considered or returned

#### Scenario: Model supplies a workspace field
- **WHEN** a model includes a folder, workspace id, project, or scope field in a `search_code` call
- **THEN** the runtime SHALL ignore the unsupported field
- **AND** it SHALL continue to use the current session workspace

#### Scenario: Embedding is unavailable after local indexing
- **WHEN** local FTS data exists but vector embedding is unconfirmed or temporarily unavailable
- **THEN** `search_code` SHALL return keyword results marked `degraded: keyword_only`
- **AND** generation SHALL continue without a tool error

#### Scenario: More source context is needed
- **WHEN** the model needs lines surrounding a code hit
- **THEN** it SHALL be able to pass the returned relative path and line range to the existing workspace-bounded `read_file` tool

### Requirement: Workspace index lifecycle retains data until explicit deletion
The system SHALL retain a workspace's configuration and index when its last active view closes or indexing is disabled. Rebuild SHALL affect only the selected workspace, and delete SHALL require confirmation and remove only the selected workspace's code-index data.

#### Scenario: Workspace closes
- **WHEN** no active view remains for an indexed workspace
- **THEN** the system SHALL stop new background work for that workspace
- **AND** it SHALL retain its manifest, chunks, symbols, and vectors for later reuse

#### Scenario: Rebuild one workspace
- **WHEN** a user requests rebuild for one workspace
- **THEN** the system SHALL preserve its configuration and invalidate only that workspace's files and code chunks

#### Scenario: Delete one workspace index
- **WHEN** a user confirms deletion of one workspace index
- **THEN** the system SHALL remove that workspace's configuration, confirmation, manifest, symbols, chunks, vectors, and local audit rows
- **AND** other workspace indexes and agent memories SHALL remain unchanged

### Requirement: Workspace code index status is phased and observable
The system SHALL expose per-workspace phase, file and chunk counts, redaction count, estimated requests, last safe failure category, and update timestamps. The frontend SHALL expose configuration, status, confirmation, rebuild, and delete through the shared service interface with Tauri and Web/mock adapter parity.

#### Scenario: Initial indexing progresses
- **WHEN** a workspace moves through scanning, parsing, confirmation, and embedding
- **THEN** the UI SHALL show the current phase and internally consistent processed and total counts

#### Scenario: Web runtime manages a mock workspace index
- **WHEN** the same service methods are used in Web runtime
- **THEN** the adapter SHALL return the same contract shape and deterministic observable transitions
- **AND** it SHALL perform no filesystem reads or embedding network calls

### Requirement: Code-index diagnostics protect private paths and content
The system SHALL route native diagnostics through unified logging and SHALL log only safe workspace ids, source kinds, phases, counts, durations, model ids, and reason categories. File-level audit records SHALL remain local to SQLite and SHALL use normalized relative paths without code content.

#### Scenario: File is skipped as sensitive
- **WHEN** a mandatory filter skips a file
- **THEN** unified logs SHALL record only the workspace id and safe skip category or aggregate count
- **AND** they SHALL NOT contain the private path or file content

#### Scenario: User inspects local audit history
- **WHEN** the UI requests code-index audit records for a workspace
- **THEN** the native service SHALL return bounded local metadata for that workspace only
- **AND** it SHALL NOT return source content, detected secret values, or paths outside that workspace

### Requirement: Workspace index mode controls external processing
The system SHALL persist each enabled workspace code index in either `local` or `semantic` mode. Workspaces persisted before mode support SHALL migrate to `semantic` to preserve their existing behavior, while newly registered workspaces SHALL use the mode selected by automatic discovery or manual configuration.

#### Scenario: New workspace uses local mode
- **WHEN** automatic discovery or manual configuration registers a workspace in local mode
- **THEN** Tree-sitter parsing and FTS5 indexing SHALL be enabled for that workspace
- **AND** no Embedding configuration SHALL be required

#### Scenario: Existing workspace is migrated
- **WHEN** the database migration encounters a workspace created before mode support
- **THEN** that workspace SHALL use `semantic` mode
- **AND** its prior confirmation and vector lifecycle SHALL remain compatible

#### Scenario: Local workspace finishes parsing
- **WHEN** Tree-sitter reconciliation completes for an enabled local workspace
- **THEN** its local index SHALL enter `ready`
- **AND** it SHALL have no pending or estimated external Embedding work
- **AND** no code or query content SHALL be sent to an Embedding provider

#### Scenario: Semantic workspace finishes parsing
- **WHEN** Tree-sitter reconciliation completes for an enabled semantic workspace
- **THEN** its local index SHALL be available for FTS5 search
- **AND** its semantic channel SHALL report whether configuration or confirmation is still required
- **AND** external processing SHALL not start before workspace-specific confirmation

#### Scenario: Workspace switches modes
- **WHEN** a user changes an enabled workspace between local and semantic modes
- **THEN** the system SHALL preserve unchanged manifests, chunks, symbols, FTS data, and existing vectors
- **AND** it SHALL invalidate stale in-flight work through the workspace generation

#### Scenario: Workspace is disabled
- **WHEN** a workspace effective mode is changed to disabled
- **THEN** the system SHALL stop scheduling parsing and retrieval work for that workspace
- **AND** it SHALL retain its configuration and existing index data for later reuse

### Requirement: OnePiece defines an automatic project indexing policy
The system SHALL expose a OnePiece automatic indexing policy with `disabled`, `local`, and `semantic` choices for newly discovered local session projects. The policy SHALL be stored independently from the OnePiece chat provider and Embedding provider profile.

#### Scenario: Automatic indexing defaults to disabled
- **WHEN** a user has not selected an automatic indexing policy on a fresh installation
- **THEN** the effective automatic policy SHALL be `disabled`
- **AND** creating a session SHALL not begin scanning its project

#### Scenario: User selects local automatic indexing
- **WHEN** a user selects `local` as the automatic policy
- **THEN** a newly discovered OnePiece project SHALL be registered in local mode
- **AND** no Embedding source or model SHALL be required

#### Scenario: User selects semantic automatic indexing
- **WHEN** a user selects `semantic` as the automatic policy
- **THEN** a newly discovered OnePiece project SHALL be registered in semantic mode
- **AND** the UI SHALL identify the effective Embedding source and model or explain that semantic enhancement is not configured

#### Scenario: Automatic policy changes after workspaces exist
- **WHEN** a user changes the automatic policy after one or more workspaces have explicit configurations
- **THEN** the system SHALL apply the new value to subsequently discovered projects
- **AND** it SHALL not silently rewrite existing workspace modes or external-send consent

### Requirement: Local OnePiece sessions discover code-index workspaces
The system SHALL asynchronously register or reuse the canonical local project folder after successfully creating a session for stable agent ID `onepiece` when the automatic policy is not disabled.

#### Scenario: Session creates an automatic workspace
- **WHEN** a OnePiece session is successfully created with a local project folder that has no workspace record
- **THEN** native orchestration SHALL canonicalize and register that folder using the automatic policy
- **AND** it SHALL enqueue reconciliation without requiring the user to add the folder in settings

#### Scenario: Session creation does not wait for indexing
- **WHEN** automatic workspace registration, parsing, or Embedding is slow or fails
- **THEN** the successfully created session SHALL remain available
- **AND** the indexing outcome SHALL be reported through status and unified logging

#### Scenario: Multiple sessions reuse one workspace
- **WHEN** multiple OnePiece sessions select paths that resolve to the same canonical folder
- **THEN** the system SHALL reuse one stable workspace record and index generation
- **AND** it SHALL not enqueue duplicate reconciliation for the same unchanged generation

#### Scenario: Existing workspace configuration is reused
- **WHEN** a OnePiece session selects a folder that already has a workspace record
- **THEN** the system SHALL retain that workspace's enabled state, mode, roots, languages, exclusions, and consent
- **AND** it SHALL reconcile only files whose indexed state is stale

#### Scenario: Automatic policy is disabled
- **WHEN** a OnePiece session is created while the automatic policy is disabled and the folder has no workspace record
- **THEN** the system SHALL not register or scan that project automatically

#### Scenario: Session uses a Git worktree
- **WHEN** the session project folder is a Git worktree
- **THEN** the system SHALL scope the index to the actual worktree folder
- **AND** it SHALL not substitute the parent repository path

#### Scenario: Session uses a remote workspace
- **WHEN** a OnePiece session uses an SSH or other remote workspace
- **THEN** the local code index SHALL not register or scan the remote path
- **AND** the session SHALL remain usable

### Requirement: Local code search is a complete retrieval mode
The system SHALL execute local-mode `search_code` requests with workspace-scoped FTS5 candidates only and SHALL NOT invoke a query Embedding provider. Keyword results in deliberate local mode SHALL be treated as successful local results rather than an unavailable semantic channel.

#### Scenario: Local search finds a symbol
- **WHEN** `search_code` matches a Tree-sitter chunk in a local-mode current-session workspace
- **THEN** it SHALL return the redacted snippet, relative path, line range, language, symbol metadata, and `matched_via: keyword`
- **AND** it SHALL NOT return a degradation marker solely because vectors are disabled by mode

#### Scenario: Semantic search is not configured
- **WHEN** a semantic-mode workspace has ready FTS5 data but no configured or confirmed vector channel
- **THEN** `search_code` SHALL continue to return workspace-scoped keyword results
- **AND** the semantic channel SHALL report its unavailable reason without marking the local index unavailable

#### Scenario: Semantic search temporarily loses a vector channel
- **WHEN** a configured semantic-mode workspace temporarily cannot use its vector channel
- **THEN** `search_code` SHALL continue to return keyword results marked `degraded: keyword_only`

### Requirement: Index status distinguishes local readiness and semantic enhancement
The system SHALL expose workspace-scoped progress and separate observable state for the local Tree-sitter/FTS5 index and optional semantic enhancement.

#### Scenario: Local workspace is ready without Embedding
- **WHEN** an enabled local workspace completes parsing
- **THEN** the status SHALL identify local mode and local readiness
- **AND** indexed chunks SHALL be greater than zero when admissible code exists
- **AND** pending chunks and estimated Embedding requests SHALL both be zero
- **AND** no confirmation-required state SHALL be shown

#### Scenario: Semantic workspace lacks a model
- **WHEN** a semantic workspace completes local parsing without an effective Embedding source and model
- **THEN** the local channel SHALL report ready
- **AND** the semantic channel SHALL report unconfigured
- **AND** the user SHALL be directed to configure Embedding without losing local search

#### Scenario: Indexing is in progress
- **WHEN** automatic reconciliation is scanning or parsing a workspace
- **THEN** status SHALL expose total and processed file counts, total and processed chunk counts, pending work, and the latest failure category

### Requirement: Global parameters and session information have separate ownership
The UI SHALL expose the OnePiece automatic policy and conditional Embedding parameters from a dedicated OnePiece page in CLI Parameter Management. Workspace-specific status and index management SHALL be shown in the active session information panel through the shared service interface. Equivalent deterministic behavior SHALL exist in the Web/mock adapter.

#### Scenario: Parameter management excludes index status
- **WHEN** the user opens the OnePiece page in CLI Parameter Management
- **THEN** it SHALL show retrieval mode and conditional Embedding parameters
- **AND** it SHALL not show index status, progress, or rebuild actions
- **AND** those workspace-scoped details SHALL remain available from the active session information panel

#### Scenario: OnePiece parameters match the managed CLI presentation
- **WHEN** the user selects OnePiece from CLI Parameter Management
- **THEN** its settings SHALL use the same parameter-card layout, control sizing, and responsive information hierarchy as the managed CLI parameter pages
- **AND** mode-specific guidance SHALL remain visible without introducing a nested settings panel

#### Scenario: User configures automatic indexing
- **WHEN** the user opens CLI Parameter Management and selects OnePiece
- **THEN** the UI SHALL offer accessible `disabled`, `local`, and `semantic` policy choices
- **AND** it SHALL explain that the choice applies automatically when new OnePiece sessions select local project folders

#### Scenario: User configures local mode
- **WHEN** a user selects local mode globally or for a workspace
- **THEN** the UI SHALL state that Tree-sitter and FTS5 remain on-device
- **AND** it SHALL not require an Embedding source or model to save or reach local-ready state

#### Scenario: User configures semantic mode
- **WHEN** a user selects semantic mode globally or for a workspace
- **THEN** the UI SHALL identify the effective Embedding source and model or explain that they must be configured
- **AND** external Embedding SHALL still require workspace-specific confirmation

#### Scenario: User reviews the active workspace status
- **WHEN** the user opens the code-index tab in the active local OnePiece session information panel
- **THEN** it SHALL resolve the session's effective worktree or project folder and show only its workspace index
- **AND** it SHALL show effective mode, local and semantic state, file/chunk progress, pending work, estimated requests, update time, and failures
- **AND** it SHALL support configuration, confirmation, refresh, rebuild, disable, and delete actions for that workspace
- **AND** switching sessions SHALL switch the displayed workspace index

#### Scenario: User deletes the active workspace index
- **WHEN** the user confirms deletion from the active session information panel
- **THEN** the confirmation dialog SHALL close without waiting for SQLite cleanup to finish
- **AND** the native deletion SHALL run outside the UI and asynchronous command executor threads
- **AND** duplicate workspace actions SHALL remain disabled until deletion settles
- **AND** completion SHALL refresh the workspace state while failure SHALL remain visible in the information panel

#### Scenario: User views the active session
- **WHEN** the active OnePiece session has a local project folder with a workspace record
- **THEN** the session UI SHALL expose a code-index information tab with effective mode and indexing progress
- **AND** full workspace configuration and management SHALL remain scoped to that session workspace

#### Scenario: Agent configuration remains provider-focused
- **WHEN** the user opens OnePiece in Agent Configuration
- **THEN** provider profile management SHALL be available
- **AND** retrieval parameters and workspace index management SHALL not be duplicated there

#### Scenario: Web runtime creates a session
- **WHEN** the Web/mock adapter creates a OnePiece session with a mock local project under an enabled automatic policy
- **THEN** it SHALL return the same workspace mode and observable transition contract as the desktop adapter
- **AND** it SHALL perform no filesystem or network operation

### Requirement: Successful Agent file mutations trigger targeted index reconciliation
The system SHALL publish the normalized relative path after a successful native Agent file write or scoped edit and SHALL asynchronously offer that path to the enabled code index for the same canonical workspace. Duplicate pending paths SHALL be coalesced, reconciliation SHALL preserve workspace-generation cancellation and existing admission rules, and notification failure SHALL NOT change the successful file-tool outcome.

#### Scenario: Agent edits an indexed source file
- **WHEN** a native Agent successfully edits a source file in an enabled code-index workspace
- **THEN** the code-index worker SHALL receive that normalized relative path for targeted reconciliation
- **AND** reconciliation SHALL remove or replace stale chunks, symbols, FTS entries, and vectors according to the current file content

#### Scenario: Agent writes in an unindexed workspace
- **WHEN** a native Agent successfully writes a file in a workspace whose code index is absent or disabled
- **THEN** the mutation notification SHALL perform no code-index work
- **AND** the file write SHALL remain successful

#### Scenario: Mutation queue already contains the path
- **WHEN** repeated successful edits publish the same workspace path before reconciliation begins
- **THEN** the background mutation queue SHALL coalesce the duplicate path without blocking the Agent tool thread

#### Scenario: Targeted reconciliation fails
- **WHEN** code-index storage or parsing fails after a successful Agent file mutation
- **THEN** the index SHALL record its existing safe degraded or audit state
- **AND** the completed file mutation SHALL NOT be retroactively changed into an Agent tool error
