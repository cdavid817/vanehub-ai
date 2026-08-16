## ADDED Requirements

### Requirement: Retrieval provides bounded Context Engine candidates
Workspace code and cross-session memory retrieval SHALL expose bounded candidate results with source provenance, workspace-relative ranges, score inputs, token estimates, and safe fingerprints through a published contract, and retrieval failure SHALL remain a non-fatal enhancement failure.

#### Scenario: Context Engine requests workspace evidence
- **WHEN** an admitted session workspace has an available local or semantic index
- **THEN** retrieval SHALL return bounded candidates without directly constructing provider prompt text

#### Scenario: Retrieval is stale or unavailable
- **WHEN** indexing is stale, disabled, or failed
- **THEN** retrieval SHALL return explicit bounded provenance or degradation
- **AND** the Context Engine SHALL remain able to use other sources
