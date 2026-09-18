## MODIFIED Requirements

### Requirement: Multi-source collection reuses authoritative capabilities
The engine SHALL normalize bounded candidates from explicit file references, retrieval and Tree-sitter symbols, LSP definitions/references or call relations when supported, relevant tests, recent workspace changes, cross-session memory, and authoritative plan/task state through existing owning-context contracts. Cross-session memory collection SHALL require the same native read context resolved for generation injection before assemble begins. Temporary, disabled, unsupported or unresolved memory contexts MUST skip that source search and query embedding while leaving other admitted sources available.

#### Scenario: Function bug gathers related evidence
- **WHEN** a user asks about a function and available sources identify its definition, callers, and relevant tests
- **THEN** those results SHALL enter one normalized candidate pipeline with source provenance

#### Scenario: LSP is unavailable
- **WHEN** an LSP source is warming, unavailable, timed out, or failed
- **THEN** retrieval and Tree-sitter candidates SHALL remain eligible
- **AND** the LSP outcome SHALL NOT fail the generation

#### Scenario: Context Engine runs with memory disabled
- **WHEN** a generation has Context Engine enabled but its memory read context is temporary or disabled
- **THEN** the engine SHALL collect no memory candidates and SHALL make no memory search or query-embedding call while continuing permitted code and task sources

### Requirement: Candidates use one bounded internal model
Each candidate SHALL have a stable id, source kind and reference, lazy or bounded content reference, token estimate with quality, relevance inputs, freshness, authority, redundancy group, required/protected flags, safe fingerprint, and allowlisted metadata; sources MUST NOT append free-form provider text directly. Memory candidates SHALL preserve internal immutable id, revision/hash and subject/scope provenance through ranking, projection and host-managed reinjection. Content hash alone SHALL not substitute for memory identity or read authority. Protected/required flags MUST NOT override eligibility.

#### Scenario: Source returns unsafe provenance
- **WHEN** a source result cannot be confined to its session/workspace or normalized within limits
- **THEN** the result SHALL be rejected with a bounded reason code
- **AND** its content SHALL NOT reach the provider or manifest

#### Scenario: Memory provenance belongs to another seat
- **WHEN** a reused memory candidate was authorized for another Agent or seat
- **THEN** the engine SHALL revalidate it under the current read context and SHALL discard it if no longer admitted

### Requirement: Evidence projection is compact and verifiable
The provider request SHALL receive only selected evidence content with compact source type, workspace-relative path, line range, symbol, and reason labels. The engine SHALL verify range validity, protected fingerprints, deduplication, and final occupancy before installing the projection. Before new provider delivery or host rebuilding of identifiable memory fragments, the engine SHALL revalidate their pinned authority and record state. It SHALL omit invalid fragments rather than restore them from a cached projection or generic fallback. This does not claim erasure of content already delivered to a provider or held in CLI-owned history.

#### Scenario: Projection verification fails
- **WHEN** any protected fingerprint, range, or budget invariant fails
- **THEN** the engine SHALL discard the candidate projection
- **AND** generation SHALL continue through the existing safe request path without partial evidence injection
