## ADDED Requirements

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
The system SHALL persist an agent memory without requiring its retrieval index entry to be written in the same operation.

#### Scenario: Indexing backend unavailable at save time
- **WHEN** a memory is saved while the embedding provider is unreachable
- **THEN** the save SHALL succeed
- **AND** the memory SHALL become searchable by keyword immediately and by vector once background indexing converges

### Requirement: Vector recall only compares same-model embeddings
The system SHALL restrict vector recall to rows whose stored embedding model equals the currently configured embedding model.

#### Scenario: Embedding model changed
- **WHEN** the configured embedding model differs from a row's stored embedding model
- **THEN** that row SHALL be excluded from vector recall
- **AND** that row SHALL remain reachable through the keyword path
- **AND** the system SHALL re-queue that row for background re-indexing

### Requirement: Retrieval tool is registered only when configured
The system SHALL offer the recall tool to the model only when an embedding source is configured.

#### Scenario: No embedding configured
- **WHEN** no embedding source is configured
- **THEN** the recall tool SHALL NOT appear in the tool catalog
- **AND** existing recency-based memory injection SHALL continue unchanged

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
