## ADDED Requirements

### Requirement: LSP provides optional Context Engine candidates
Trusted and ready LSP definitions, references, and supported call relations SHALL be normalizable as bounded Context Engine candidates with server, language, document-version, range, truncation, and stale-state provenance.

#### Scenario: LSP returns definition and references
- **WHEN** a planned symbol query completes within existing LSP bounds
- **THEN** normalized locations SHALL enter the Context Engine candidate pipeline rather than append provider text directly

#### Scenario: LSP cannot serve the request
- **WHEN** trust, capability, readiness, timeout, cancellation, or server failure prevents a query
- **THEN** the source SHALL return its existing bounded degradation state
- **AND** it SHALL NOT fail candidate collection or generation
