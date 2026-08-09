## ADDED Requirements

### Requirement: Retrieval indexing applies source-specific scope semantics
The retrieval system SHALL require each indexing, queue, status, rebuild, deletion, and search operation to identify a source kind and its valid scope. `agent_memory` SHALL retain its host-wide shared-pool behavior, while `workspace_file` SHALL require a workspace id and SHALL never query another workspace.

#### Scenario: Agent memory is recalled after source generalization
- **WHEN** recall searches `agent_memory` after workspace code indexing is enabled
- **THEN** it SHALL continue to consider memories from every agent and folder
- **AND** its tool schema and payload SHALL remain unchanged

#### Scenario: Workspace candidate query is executed
- **WHEN** hybrid search requests `workspace_file` candidates for one workspace id
- **THEN** both vector and keyword queries SHALL filter by that workspace before ranking

### Requirement: Reconciliation and batching are parameterized by source
The retrieval application SHALL reconcile and claim pending documents using an explicit source kind and scope rather than a hard-coded `AgentMemory`. Processing one source or workspace SHALL NOT invalidate, delete, claim, or update another source or workspace.

#### Scenario: Code file reconcile runs
- **WHEN** the indexing service reconciles changed files for one workspace
- **THEN** it SHALL upsert and remove only `workspace_file` documents belonging to that workspace
- **AND** agent-memory documents SHALL remain unchanged

#### Scenario: Memory worker runs with code pending
- **WHEN** the memory worker claims an `agent_memory` batch while code chunks are pending
- **THEN** it SHALL claim only memory documents

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
