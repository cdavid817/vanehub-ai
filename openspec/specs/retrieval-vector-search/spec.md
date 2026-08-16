# retrieval-vector-search Specification

## Purpose
Defines meaning-based recall over the shared memory pool and over indexed workspace code, including how each source is scoped, reconciled, and kept from surfacing content that no longer exists. Retrieval is an enhancement: its failure degrades recall, never a generation.
## Requirements
### Requirement: Retrieval searches the shared host-level memory pool
The system SHALL search the same host-level memory pool that recency-based memory injection draws from (`agent-memory-shared-pool`), and SHALL NOT restrict recall by agent id or workspace folder. Agent id and workspace folder SHALL be recorded on an index row as provenance only, and SHALL NOT be exposed as recall tool input.

#### Scenario: Memory saved under a different agent is recallable
- **WHEN** the model invokes the recall tool from one agent's session
- **THEN** the system SHALL consider memories saved under every other agent and every workspace folder
- **AND** recall SHALL NOT return a strict subset of what memory injection already placed in the system prompt

#### Scenario: Recall tool exposes no scope parameter
- **WHEN** the recall tool definition is resolved
- **THEN** its input schema SHALL expose exactly `query` and `limit`
- **AND** it SHALL NOT expose an agent id, folder, or any other scope parameter, because the shared pool has no slice for the model to name

### Requirement: Retrieval failure never fails generation
The system SHALL return a successful tool result describing unavailability when retrieval fails, and SHALL NOT surface retrieval failure as a generation error.

#### Scenario: Embedding provider unreachable during search
- **WHEN** query embedding fails while retrieval is configured
- **THEN** the system SHALL return keyword-only results marked `degraded: keyword_only`

#### Scenario: Keyword path fails
- **WHEN** the FTS5 query fails
- **THEN** the system SHALL return vector-only results marked `degraded: vector_only`

#### Scenario: Both paths yield nothing
- **WHEN** both paths execute successfully and neither returns a hit
- **THEN** the system SHALL return an empty result list and SHALL NOT report an error

#### Scenario: Both paths fail
- **WHEN** both the vector path and the keyword path fail
- **THEN** the system SHALL report retrieval as unavailable rather than as an empty result set
- **AND** the recall tool SHALL still return a successful tool result so that generation continues

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

### Requirement: Vector recall only compares same-model embeddings
The system SHALL restrict vector recall to rows whose stored embedding model equals the currently configured embedding model.

#### Scenario: Embedding model changed
- **WHEN** the configured embedding model differs from a row's stored embedding model
- **THEN** that row SHALL be excluded from vector recall
- **AND** that row SHALL remain reachable through the keyword path
- **AND** the system SHALL re-queue that row for background re-indexing

### Requirement: Retrieval tool is registered only when configured

The system SHALL offer the recall tool to the model only when an embedding source is configured. Memory injection SHALL NOT depend on that configuration: with no embedding source configured, the memory index SHALL still be injected and relevance selection SHALL still run, so that an installation without retrieval keeps a working memory feature.

#### Scenario: No embedding configured

- **WHEN** no embedding source is configured
- **THEN** the recall tool SHALL NOT appear in the tool catalog
- **AND**, replacing this scenario's previous guarantee that recency-based memory injection continues unchanged, index injection and relevance-selected body injection SHALL both continue to operate

#### Scenario: Embedding configured

- **WHEN** an embedding source is configured
- **THEN** the recall tool SHALL appear in the tool catalog
- **AND** it SHALL remain the content-driven search path, complementary to the description-driven relevance selection rather than replaced by it

### Requirement: Retrieval logging excludes sensitive content
The system SHALL NOT persist memory content, raw query text, credentials, or provider response bodies to logs.

#### Scenario: Query logged for diagnostics
- **WHEN** a retrieval executes
- **THEN** the system SHALL log only the query's length and hash alongside candidate count, per-path hit counts, and duration

### Requirement: Web runtime contract parity
The Web/mock runtime SHALL expose the same retrieval contract shape and observable behavior as the desktop runtime, and SHALL NOT issue network requests.

#### Scenario: Web runtime search
- **WHEN** retrieval is invoked in the Web/mock runtime
- **THEN** it SHALL return the same result structure, the same degraded semantics, and treat empty results as success
- **AND** it MAY rank by a simple term-overlap score rather than reproducing vector similarity

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

### Requirement: Retrieval status and rebuild support explicit source scopes
The retrieval system SHALL provide global agent-memory status for backward compatibility and SHALL provide per-workspace code status, rebuild, and delete operations. Aggregate UI status SHALL be derived from scoped results rather than by weakening workspace isolation.

#### Scenario: Read existing memory status
- **WHEN** the existing retrieval status command is invoked
- **THEN** it SHALL report the same host-wide agent-memory counts as before this change

#### Scenario: Rebuild code workspace
- **WHEN** code-index rebuild is requested for one workspace id
- **THEN** only that workspace's `workspace_file` documents SHALL be requeued

### Requirement: Embedding model invalidation is source and scope aware
The retrieval system SHALL exclude embeddings created by a different model and SHALL requeue stale rows in bounded source-scoped batches. A code model change SHALL NOT force unrelated workspaces or agent memories to rebuild unless they use the changed effective configuration.

#### Scenario: Workspace embedding model changes
- **WHEN** the effective embedding model changes for a confirmed workspace code index
- **THEN** that workspace's old-model vectors SHALL be excluded immediately
- **AND** its redacted chunks SHALL await renewed confirmation before bounded re-embedding

#### Scenario: Agent-memory model remains current
- **WHEN** a code workspace changes model but the global agent-memory model does not
- **THEN** indexed agent-memory rows SHALL remain indexed and searchable

### Requirement: Retrieval provides bounded Context Engine candidates
Workspace code and cross-session memory retrieval SHALL expose bounded candidate results with source provenance, workspace-relative ranges, score inputs, token estimates, and safe fingerprints through a published contract, and retrieval failure SHALL remain a non-fatal enhancement failure.

#### Scenario: Context Engine requests workspace evidence
- **WHEN** an admitted session workspace has an available local or semantic index
- **THEN** retrieval SHALL return bounded candidates without directly constructing provider prompt text

#### Scenario: Retrieval is stale or unavailable
- **WHEN** indexing is stale, disabled, or failed
- **THEN** retrieval SHALL return explicit bounded provenance or degradation
- **AND** the Context Engine SHALL remain able to use other sources

