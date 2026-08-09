## ADDED Requirements

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
