## ADDED Requirements

### Requirement: Workspace inspection concurrency and retained work are structurally bounded

The runtime SHALL enforce finite global and per-workspace active inspection limits, a finite admission wait/queue policy, finite operation budgets, and bounded retained candidate/page state before starting blocking or remote work. Admission ownership SHALL continue until the actual worker exits, including after caller cancellation.

#### Scenario: Repeated searches supersede faster than workers exit

- **WHEN** a view repeatedly starts the same search id while earlier blocking workers are still stopping
- **THEN** each newer generation SHALL cancel the prior generation
- **AND** active workers SHALL never exceed the configured global/per-workspace admission limits
- **AND** requests beyond finite admission capacity SHALL receive a typed busy result rather than creating an unbounded queue

#### Scenario: Structural memory gate for directory pagination

- **WHEN** an instrumented directory provider exposes a number of entries far above the requested page size
- **THEN** the test SHALL observe retained page candidates bounded by page size plus fixed overhead
- **AND** the assertion SHALL not depend on absolute process memory or shared-CI latency

#### Scenario: Cancellation checkpoint gate

- **WHEN** cancellation is signalled to an instrumented traversal or file reader
- **THEN** the worker SHALL stop within the configured maximum number of additional entry/chunk checkpoints
- **AND** the test SHALL not require a platform-specific millisecond threshold to prove correctness
