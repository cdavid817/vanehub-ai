## ADDED Requirements

### Requirement: Run lifecycle performance is bounded and measurable
The canonical Run benchmark SHALL cover event propagation, valid state transition overhead, terminal idempotency, cancellation latency, token/progress event batching, and concurrent resource growth using the existing Run identity and lifecycle rules.

#### Scenario: One thousand Run histories are exercised
- **WHEN** the versioned 1,000-Run dataset applies deterministic lifecycle and cancellation sequences
- **THEN** transition work, retained events, query pages, and update batches SHALL remain within declared structural budgets
- **AND** illegal or duplicate terminal transitions SHALL retain their existing safe outcomes

#### Scenario: Concurrent runs are measured
- **WHEN** the dedicated benchmark increases supported concurrent Runs
- **THEN** it SHALL record throughput, cancellation latency, and resource growth with platform and build-profile provenance
- **AND** shared CI SHALL enforce only declared concurrency, buffer, and item-count bounds

