## ADDED Requirements

### Requirement: Agent Runner resource budgets are deterministic and bounded
The runtime performance harness SHALL measure Local and fake SSH Runner admission, event buffering, concurrent Run registry growth, pooled transport reuse, cancellation, disconnect/reconnect attempts, and cleanup using versioned fixtures and structural budgets. Shared CI MUST enforce declared counts and capacities rather than fixed wall-clock latency.

#### Scenario: Concurrent Runner fixture executes
- **WHEN** the versioned fixture increases Local and SSH Runs to the supported concurrency limit
- **THEN** active handles, threads or tasks, channels, pooled transports, queued events, retained bytes, reconnect attempts, and cleanup records remain within declared budgets

#### Scenario: Resource regression fixture exceeds one bound
- **WHEN** the negative fixture leaks a handle, grows an unbounded event queue, or establishes one SSH transport per compatible Run
- **THEN** the deterministic comparator fails with metric, dataset, baseline, measured value, and budget

