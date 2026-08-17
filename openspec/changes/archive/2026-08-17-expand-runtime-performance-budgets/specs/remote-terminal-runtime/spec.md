## ADDED Requirements

### Requirement: Remote terminal performance evidence is bounded
The terminal benchmark SHALL cover a versioned long-output dataset, bounded UTF-8 chunks, retained buffer capacity, indexed search pages, cancellation, and dropped-content gap behavior without retaining raw terminal content in result records.

#### Scenario: Long terminal history is searched
- **WHEN** the versioned long-terminal dataset is captured and searched
- **THEN** chunk size, retained bytes, loaded rows, query count, and result page size SHALL remain within deterministic budgets
- **AND** P50/P95 latency SHALL be recorded only as dedicated evidence

#### Scenario: Terminal dataset exceeds safety limits
- **WHEN** fixture content or requested result size exceeds its declared bound
- **THEN** the harness SHALL reject or truncate it according to the existing terminal contract and record only bounded counts and reason codes

