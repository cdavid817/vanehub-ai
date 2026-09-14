## ADDED Requirements

### Requirement: One governed eligibility domain for every memory read
All VaneHub-owned Agent memory summaries, index lines, relevance-selection inputs, selected bodies, recall hits, Context Engine candidates and identifiable host reinjection fragments SHALL use the same eligibility rules before ranking, selection, budgeting or delivery. Eligibility MUST require valid active authoritative records, a healthy trusted read context, effective memory-read permission, admitted global/workspace scope and exact Agent audience membership. Provenance MUST NOT grant access. Storage MAY remain one host-level pool; consumers MAY choose different ranked subsets.

#### Scenario: Standard session reads admitted global and workspace records
- **WHEN** an Agent in a valid workspace uses standard mode with read and global-memory access enabled
- **THEN** global and matching-workspace active records SHALL be eligible only when their audience admits the stable Agent id; other workspaces and inactive records SHALL remain excluded

#### Scenario: Project-only and disabled global access narrow the pool
- **WHEN** project-only mode or effective global-memory denial applies
- **THEN** global records SHALL be excluded from every memory read surface while matching-workspace admitted records SHALL remain readable

#### Scenario: Temporary or disabled memory has no collection side effects
- **WHEN** the effective context is temporary or memory read is disabled
- **THEN** the runtime SHALL skip memory-source search, query embedding, relevance selection and memory-body loading while allowing generation and unrelated sources to continue

#### Scenario: Shared storage does not make every record public
- **WHEN** a record was produced by the current Agent but its explicit scope or audience excludes that Agent context
- **THEN** the runtime SHALL exclude it without treating the producing Agent or folder as authorization

### Requirement: Native memory read context cannot be supplied by the model
A versioned MemoryReadContext SHALL be constructed by the native session/generation owner and personalization resolver before any memory collection. It MUST bind stable Agent identity, session, generation/seat-turn ownership, actual workspace identity or explicit absence, session mode, frozen effective policy revision, read/global/workspace allowances and current capability/maintenance provenance. All Agent read ports MUST require that context; missing context MUST NOT mean owner or unrestricted access. The public recall input SHALL remain exactly query and limit.

#### Scenario: Tool input invents another scope
- **WHEN** a model adds agentId, workspace, scope, owner or an arbitrary allowed-id list to a recall request
- **THEN** the runtime SHALL ignore or reject the extra properties without changing the trusted subject or widening the read domain

#### Scenario: Workspace cannot be resolved
- **WHEN** the actual session has workspace context that cannot be resolved safely
- **THEN** memory reads SHALL fail closed rather than treat the failure as a standard session with no workspace and allow global data

#### Scenario: Standard session intentionally has no workspace
- **WHEN** the native session explicitly has no workspace and otherwise has a valid standard context
- **THEN** only global records admitted by its effective policy and audience MAY be read

#### Scenario: Another seat supplies an old context
- **WHEN** a request reuses another Agent/seat/workspace context or a context belonging to an ended generation or prior application epoch
- **THEN** the native read boundary SHALL reject it before collection

### Requirement: Full eligibility is independent of injection limits
The frozen context SHALL describe complete eligibility conditions, not the bounded injection reference page. Recall and Context Engine queries MUST consider the complete indexed eligible domain before their own ranking and limits. A display, selector or token cap MUST NOT permanently exclude an otherwise eligible memory from recall. Source-specific limits SHALL remain bounded and observable.

#### Scenario: Relevant memory is beyond the first two hundred refs
- **WHEN** a highly relevant eligible record is absent from the snapshot injection page because that page reached its 200-reference bound
- **THEN** recall SHALL still be able to return it and the injection surface SHALL report its own truncation

#### Scenario: Injection and recall choose different results
- **WHEN** consumers use different queries, ordering, surfaced suppression or budgets over the same eligible corpus
- **THEN** their returned subsets MAY differ without being classified as an authorization mismatch

### Requirement: Authorized candidates precede hybrid ranking
Both keyword and vector retrieval SHALL use the same complete authorized metadata relation before path ranking and top-k. The relation MUST be produced through the personalization owning contract with exact eligibility semantics and bounded, consistent metadata paging, without loading every body or querying another context private tables. Retrieval SHALL filter vector rows before loading/scoring and keyword rows before rank/LIMIT. Incomplete relations MUST fail closed, never fall back to a public or owner-wide pool. Query-local authority MUST be isolated and cleaned up on completion, cancellation or error.

#### Scenario: Unauthorized records dominate global similarity
- **WHEN** excluded records would fill a global top-k ahead of a valid matching memory
- **THEN** both retrieval paths SHALL exclude those records before top-k so they cannot consume the valid candidate allowance

#### Scenario: Authorization paging fails midway
- **WHEN** a consistent complete metadata relation cannot be obtained within the configured query budget
- **THEN** retrieval SHALL return no hits and report unavailable through the supported safe outcome path; it SHALL NOT search or deliver from a partial authorization relation

#### Scenario: Connection is reused across concurrent subjects
- **WHEN** one query is cancelled and its connection or temporary relation is reused by another subject
- **THEN** the next query SHALL use only its own context-bound relation and SHALL not inherit the cancelled request authority

#### Scenario: Embedding network call is slow
- **WHEN** query embedding takes variable time
- **THEN** the implementation SHALL have released the metadata snapshot transaction before that network call and SHALL retain only its bounded context-bound query data

### Requirement: Exact eligibility is validated across representations
Rust domain checks, SQL eligibility projection and Web/mock behavior SHALL agree on a versioned eligibility fixture. Selected-Agent membership MUST compare complete stable ids with exact identity semantics, not SQL LIKE, substrings or case folding. Unknown scope/status/schema, malformed audience data and inconsistent workspace metadata MUST NOT enter eligible summaries or candidates.

#### Scenario: Agent ids resemble SQL patterns
- **WHEN** valid Agent ids contain percent, underscore, quotes, prefixes or case differences
- **THEN** SQL and domain membership SHALL produce the same exact admitted id set without wildcard or case-insensitive matches

#### Scenario: Projection contains an unknown scope
- **WHEN** a projected record has an unknown scope kind or malformed audience/workspace shape
- **THEN** the record SHALL be rejected before name, description or body reaches an Agent and SHALL produce bounded safe diagnostics

### Requirement: Authoritative pinned delivery protects metadata and bodies
Every Agent-visible memory record SHALL be resolved through a governed native read API by immutable id with pinned revision, content hash and authorization-metadata fingerprint. The runtime MUST revalidate authoritative existence, v2 validity, lifecycle, scope, audience and ownership before delivery; summary/index/selector metadata MUST receive equivalent eligibility validation. A stale, replaced or revoked handle SHALL be dropped, not silently replaced with newer content, owner detail or indexed text. Metadata and body checks MUST refer to a consistent safely read file instance.

#### Scenario: Memory changes between selection and delivery
- **WHEN** the record is edited, archived, deleted, moved to another scope or assigned a different audience after its handle was pinned
- **THEN** delivery SHALL discard the stale handle without leaking the old indexed body or substituting a different current record

#### Scenario: External edit retains the old revision
- **WHEN** an external editor changes body or authority metadata without advancing the stored revision
- **THEN** hash/metadata validation SHALL detect the mismatch and SHALL not deliver the stale selection as valid

#### Scenario: Two records share a name
- **WHEN** the selector chooses one of two equally named eligible records
- **THEN** only the selected immutable id and its pinned version MAY be loaded; name-first matching SHALL NOT choose another record

#### Scenario: Index summary is no longer eligible
- **WHEN** a record is no longer admitted before its name or description is sent to the selector, provider or CLI
- **THEN** the metadata SHALL be omitted even if its body would later be rejected

### Requirement: Frozen policy and fresh record batches have explicit semantics
A generation or seat turn SHALL retain its captured personalization policy revision as required by existing governance; saving policy SHALL affect later generations rather than silently hot-swapping the active generation policy. Each new memory read batch MAY discover current records within those same frozen rules and SHALL pin and revalidate its own record handles. Current unhealthy maintenance, invalid ownership or epoch MUST block new reads. Previously delivered model or CLI content MUST NOT be described as retroactively revoked.

#### Scenario: Policy changes during a turn
- **WHEN** the user saves a different read policy while one generation is active
- **THEN** that generation SHALL keep its captured policy and later generations SHALL use the new revision; the UI SHALL accurately describe this boundary

#### Scenario: New eligible memory is approved during a turn
- **WHEN** a later recall request occurs after a new record becomes active inside the frozen allowed scope
- **THEN** the new request MAY pin and return that record under the same policy, without replacing a handle already pinned by an earlier request

#### Scenario: A cached policy is used after a transient fault
- **WHEN** the existing last-known-good fallback supplies a validated exact-context policy
- **THEN** all surfaces SHALL use the same resolved context and warning while still checking current maintenance and record validity; cache fallback SHALL not grant a different scope

### Requirement: Management indexing and runtime authority remain distinct
Owner memory management, infrastructure index maintenance and Agent runtime reading SHALL have separate typed native entry points. Runtime ports MUST NOT reach owner-wide operations by omitting context. Background local indexing SHALL cover valid active records independently of one live session eligibility, while Agent queries remain scoped. Existing compatibility views MUST NOT be broadened into an unguarded runtime fallback. Index expansion MUST NOT itself authorize additional memory-body egress to an embedding provider.

#### Scenario: Owner manages records excluded from an Agent
- **WHEN** the user searches or edits memory from the settings management interface
- **THEN** existing owner-level access SHALL remain available while the same records remain inaccessible to an excluded Agent

#### Scenario: Scoped record has no embedding egress authorization
- **WHEN** a previously unindexed workspace or selected-Agent record has no verifiable existing authorization for body transmission to the configured remote embedder
- **THEN** it SHALL remain eligible for local FTS indexing without automatic body egress or a false claim that vector indexing is complete

#### Scenario: A session loses read permission
- **WHEN** one session can no longer read an active record
- **THEN** the system SHALL not delete that record or its host-maintained local index solely because of that session restriction

### Requirement: Memory degradation never widens scope
Authorized query failures SHALL preserve existing nonfatal generation behavior. Keyword-only and vector-only fallback MUST retain the same read context and authorized relation. Missing authority, unavailable complete eligibility or unsafe maintenance MUST NOT use legacy public-pool, cached body or owner search fallback. Empty-success, disabled, unavailable and incomplete results SHALL remain distinguishable without exposing excluded-record existence to the model. Existing model result fields SHALL be preserved; unsupported completeness failures SHALL use the established successful unavailable tool response rather than invent an unversioned degradation enum.

#### Scenario: Vector path fails
- **WHEN** vector retrieval fails while authorized keyword retrieval succeeds
- **THEN** the tool SHALL return only authorized keyword results with the existing keyword_only degradation

#### Scenario: Both paths return no matches
- **WHEN** both authorized paths complete successfully and find no valid hits
- **THEN** the tool SHALL return an empty successful result

#### Scenario: Policy or complete query authority is unavailable
- **WHEN** retrieval cannot establish a safe read context or complete candidate domain
- **THEN** the tool SHALL return a safe unavailable response while generation continues and SHALL not search a broader pool

### Requirement: Derived memory fragments retain their read provenance
Host-identifiable memory candidates, tool results, surfaced markers and reinjection fragments SHALL retain internal id/version and subject/scope provenance. Cached or previously surfaced content MUST pass eligibility again before host re-delivery, context rebuilding or reuse for a different subject. Ranking flags and explicit references MUST NOT widen memory authority. This guarantee SHALL not claim control over CLI-owned internal history, human-copied text or model paraphrases.

#### Scenario: A memory candidate is marked protected
- **WHEN** a Context Engine candidate is marked explicit, required or protected but its read context does not admit the record
- **THEN** the runtime SHALL exclude it before budgeting or provider delivery

#### Scenario: A different seat rebuilds context
- **WHEN** a new seat or Agent would reuse a host-identifiable prior memory fragment
- **THEN** the host SHALL validate it under the new subject and SHALL drop any ineligible fragment rather than reuse the old authorization

#### Scenario: The CLI already received a prior prompt
- **WHEN** a policy or record changes after content was written to a CLI-owned context
- **THEN** the host SHALL govern future host deliveries and SHALL not claim that existing CLI history has been erased
