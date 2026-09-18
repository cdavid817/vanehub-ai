## MODIFIED Requirements

### Requirement: Retrieval searches the shared host-level memory pool
The system SHALL search the same host-level memory storage used by injection while applying the same lifecycle, read-policy, explicit scope and audience eligibility rules for the trusted calling generation/seat. Provenance Agent and folder SHALL not be used as authorization. The complete eligible candidate domain, not the bounded injection page or identical final result set, SHALL define recall scope. Both retrieval paths SHALL filter before ranking and top-k and every delivered hit SHALL pass authoritative pinned revalidation. Owner management search SHALL remain separate.

#### Scenario: Memory saved under a different agent is recallable
- **WHEN** the model invokes recall from a valid Agent context and a record produced by another Agent has scope and audience admitting that context
- **THEN** the system SHALL consider that record regardless of its producing Agent or source folder
- **AND** recall SHALL use the same eligibility rules as injection while query, ranking and budget MAY produce a different subset

#### Scenario: Recall tool exposes no scope parameter
- **WHEN** the recall tool definition is resolved
- **THEN** its input schema SHALL expose exactly `query` and `limit`
- **AND** it SHALL NOT expose an agent id, folder, or any other scope parameter, because only the native runtime establishes the read context

### Requirement: Retrieval failure never fails generation
The system SHALL return a successful tool result describing unavailability when retrieval fails, and SHALL NOT surface retrieval failure as a generation error. All degradation SHALL retain the trusted read context and complete authorized domain. Failed context, eligibility or authoritative-store validation MUST NOT trigger compatibility-pool, owner-wide or cached-body fallback. Incomplete authorization/search state SHALL not be represented as a completed empty search.

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

#### Scenario: Eligibility fails before keyword fallback
- **WHEN** the vector path is unavailable and the runtime cannot establish the keyword authorization relation
- **THEN** retrieval SHALL report unavailable through the existing nonfatal tool path without running an unfiltered keyword query

### Requirement: Saving a memory never depends on indexing

The system SHALL persist an agent memory without requiring its retrieval index entry to be written in the same operation. Persisting a memory means writing its file and its index-file line; neither SHALL be conditional on the retrieval index accepting the document. Index maintenance SHALL derive local searchable records for all valid active scopes and audiences through a dedicated owning API, not the compatibility public view. Permission or lifecycle changes SHALL exclude newly delivered stale records without waiting for embedding convergence. Existing save and embedding-authorization boundaries SHALL remain independent.

#### Scenario: Indexing backend unavailable at save time

- **WHEN** a memory is saved while the embedding provider is unreachable
- **THEN** the save SHALL succeed
- **AND** the memory SHALL become searchable by keyword immediately and by vector once authorized background embedding converges; records without body-egress authorization SHALL remain keyword-only

#### Scenario: Correcting a memory re-queues it

- **WHEN** an existing memory's content is replaced
- **THEN** the save SHALL succeed regardless of indexing availability
- **AND** the memory SHALL be re-queued so that authoritative revalidation immediately prevents delivery of superseded content even while indexing converges

### Requirement: Retrieval tool is registered only when configured

The system SHALL offer the recall tool to the model only when an embedding source is configured and the trusted runtime context permits memory read through that channel. Execution SHALL repeat the native context gate even if a caller bypasses the catalog. Memory injection SHALL NOT depend on that configuration: with no embedding source configured but valid read permission and the corresponding runtime capabilities, the memory index SHALL still be injected and relevance selection SHALL still run, so that an installation without retrieval keeps a working memory feature.

#### Scenario: No embedding configured

- **WHEN** no embedding source is configured and the trusted context permits the corresponding index/body injection capabilities
- **THEN** the recall tool SHALL NOT appear in the tool catalog
- **AND**, replacing this scenario's previous guarantee that recency-based memory injection continues unchanged, index injection and relevance-selected body injection SHALL both continue to operate

#### Scenario: Embedding configured

- **WHEN** an embedding source is configured and the runtime context permits governed recall
- **THEN** the recall tool SHALL appear in the tool catalog
- **AND** it SHALL remain the content-driven search path, complementary to the description-driven relevance selection rather than replaced by it

#### Scenario: Read permission is disabled with retrieval configured
- **WHEN** a session is temporary or its captured policy denies memory read
- **THEN** recall SHALL be omitted and direct invocation SHALL fail closed without memory search or query embedding

### Requirement: Retrieval indexing applies source-specific scope semantics

The retrieval system SHALL require each indexing, queue, status, rebuild, deletion, and search operation to identify a source kind and its valid scope. `agent_memory` SHALL retain host-wide storage and maintenance with per-runtime governed read filtering, while `workspace_file` SHALL require a workspace id and SHALL never query another workspace. An `agent_memory` document SHALL be identified by its memory file's path relative to the memory directory rather than by a database row id, and an Agent search hit SHALL be resolved through the governed owning API with immutable id/revision/hash and audience/scope validation before delivery. Index provenance columns MUST NOT substitute for explicit read authority.

#### Scenario: Agent memory is recalled after source generalization

- **WHEN** recall searches `agent_memory` after workspace code indexing is enabled
- **THEN** it SHALL consider the complete eligible indexed domain under the trusted runtime context, regardless of producer provenance
- **AND** its tool schema and payload SHALL remain unchanged

#### Scenario: Workspace candidate query is executed

- **WHEN** hybrid search requests `workspace_file` candidates for one workspace id
- **THEN** both vector and keyword queries SHALL filter by that workspace before ranking

#### Scenario: Hit resolves against a missing file

- **WHEN** a search hit names a memory file that no longer exists in the memory directory
- **THEN** the system SHALL omit that hit from the results rather than returning stale indexed text
- **AND** it SHALL NOT fail the recall

### Requirement: Web runtime contract parity
The Web/mock runtime SHALL expose the same retrieval contract shape and observable behavior as the desktop runtime, and SHALL NOT issue network requests. Web/mock governed memory search SHALL apply the same effective read/global policy, session narrowing, workspace and exact audience rules as native injection before ranking. It SHALL distinguish permitted empty pools, disabled read, unconfigured retrieval and unavailable sources; simulated results MUST NOT claim native execution.

#### Scenario: Web runtime search
- **WHEN** retrieval is invoked in the Web/mock runtime
- **THEN** it SHALL return the same result structure, the same degraded semantics, and treat empty results as success
- **AND** it MAY rank by a simple term-overlap score rather than reproducing vector similarity

### Requirement: Retrieval provides bounded Context Engine candidates
Workspace code and cross-session memory retrieval SHALL expose bounded candidate results with source provenance, workspace-relative ranges, score inputs, token estimates, and safe fingerprints through a published contract, and retrieval failure SHALL remain a non-fatal enhancement failure. Memory candidates SHALL require a trusted MemoryReadContext and carry internal immutable memory id, revision/hash and scope provenance. A memory source MUST NOT invoke unscoped host-pool search or expand scope through explicit/protected ranking flags.

#### Scenario: Context Engine requests workspace evidence
- **WHEN** an admitted session workspace has an available local or semantic index
- **THEN** retrieval SHALL return bounded candidates without directly constructing provider prompt text

#### Scenario: Retrieval is stale or unavailable
- **WHEN** indexing is stale, disabled, or failed
- **THEN** retrieval SHALL return explicit bounded provenance or degradation
- **AND** the Context Engine SHALL remain able to use other sources

## ADDED Requirements

### Requirement: Scoped memory indexing preserves egress authority
Local agent-memory FTS maintenance SHALL include eligible active workspace and audience-restricted records independently of any one session policy. Expanding that local index MUST NOT automatically expand authorization to transmit bodies to a remote embedding provider. At this baseline there is no scoped-memory egress consent capability, so workspace or audience-restricted records SHALL remain keyword-only until a separately specified consent capability exists; code-index confirmation MUST NOT substitute for it. Queue claim, retry, rebuild and model changes MUST respect this restriction. Before any embedding-body dispatch the worker SHALL revalidate authoritative lifecycle, scope/audience, pinned revision and metadata/content identity; a stale queued public record MUST NOT authorize later restricted content. Query-time vector filtering SHALL still use the same read domain when authorized vectors exist. Source health and reconciliation failures MUST NOT be represented as an authoritative empty source that deletes all rows.

#### Scenario: Workspace memory was absent from the old public index
- **WHEN** a valid active workspace record is reconciled by the governed indexing source
- **THEN** its local FTS entry SHALL become searchable by an admitted session even when no authorized vector is available

#### Scenario: Remote embedding is not authorized for newly included content
- **WHEN** a record newly included by scoped indexing lacks verifiable authorization for body egress
- **THEN** the worker SHALL not transmit that body merely because query embedding is configured

#### Scenario: A queued public record becomes restricted
- **WHEN** a global all-Agent memory becomes workspace-scoped or audience-restricted before a pending, retried or rebuilt embedding dispatch
- **THEN** the worker SHALL block that body dispatch, retain its local FTS availability and SHALL not count the blocked vector as a completed embedding

#### Scenario: Memory source is unavailable during reconciliation
- **WHEN** the authoritative memory source cannot be completely enumerated
- **THEN** reconciliation SHALL retain existing rows for recovery and SHALL not interpret the failure as deletion of the whole pool
