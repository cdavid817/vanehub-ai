## MODIFIED Requirements

### Requirement: Saving a memory never depends on indexing

The system SHALL persist an agent memory without requiring its retrieval index entry to be written in the same operation. Persisting a memory means writing its file and its index-file line; neither SHALL be conditional on the retrieval index accepting the document.

#### Scenario: Indexing backend unavailable at save time

- **WHEN** a memory is saved while the embedding provider is unreachable
- **THEN** the save SHALL succeed
- **AND** the memory SHALL become searchable by keyword immediately and by vector once background indexing converges

#### Scenario: Correcting a memory re-queues it

- **WHEN** an existing memory's content is replaced
- **THEN** the save SHALL succeed regardless of indexing availability
- **AND** the memory SHALL be re-queued so that recall stops matching the superseded content once indexing converges

### Requirement: Retrieval indexing applies source-specific scope semantics

The retrieval system SHALL require each indexing, queue, status, rebuild, deletion, and search operation to identify a source kind and its valid scope. `agent_memory` SHALL retain its host-wide shared-pool behavior, while `workspace_file` SHALL require a workspace id and SHALL never query another workspace. An `agent_memory` document SHALL be identified by its memory file's path relative to the memory directory rather than by a database row id, and a search hit SHALL be resolved by reading that file.

#### Scenario: Agent memory is recalled after source generalization

- **WHEN** recall searches `agent_memory` after workspace code indexing is enabled
- **THEN** it SHALL continue to consider memories from every agent and folder
- **AND** its tool schema and payload SHALL remain unchanged

#### Scenario: Workspace candidate query is executed

- **WHEN** hybrid search requests `workspace_file` candidates for one workspace id
- **THEN** both vector and keyword queries SHALL filter by that workspace before ranking

#### Scenario: Hit resolves against a missing file

- **WHEN** a search hit names a memory file that no longer exists in the memory directory
- **THEN** the system SHALL omit that hit from the results rather than returning stale indexed text
- **AND** it SHALL NOT fail the recall

### Requirement: Reconciliation and batching are parameterized by source

The retrieval application SHALL reconcile and claim pending documents using an explicit source kind and scope rather than a hard-coded `AgentMemory`. Processing one source or workspace SHALL NOT invalidate, delete, claim, or update another source or workspace. Reconciliation for `agent_memory` SHALL take a scan of the memory directory as its authoritative snapshot, so that a file added, changed, or removed outside the application converges without user action.

#### Scenario: Code file reconcile runs

- **WHEN** the indexing service reconciles changed files for one workspace
- **THEN** it SHALL upsert and remove only `workspace_file` documents belonging to that workspace
- **AND** agent-memory documents SHALL remain unchanged

#### Scenario: Memory worker runs with code pending

- **WHEN** the memory worker claims an `agent_memory` batch while code chunks are pending
- **THEN** it SHALL claim only memory documents

#### Scenario: Directory scan drives agent-memory reconciliation

- **WHEN** `agent_memory` reconciliation runs against a memory directory containing a file with no index entry, and holding an index entry whose file is gone
- **THEN** it SHALL queue the unindexed file and remove the orphaned entry
- **AND** it SHALL leave `workspace_file` documents untouched
