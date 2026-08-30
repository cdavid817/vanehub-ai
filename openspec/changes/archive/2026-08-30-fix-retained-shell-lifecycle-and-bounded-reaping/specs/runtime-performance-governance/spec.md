## ADDED Requirements

### Requirement: Shell lifecycle resource use and cleanup latency are explicitly bounded

The runtime SHALL govern Shell capacity reservations, startup event buffering, command-path close duration, Reaper queue depth, Reaper concurrency, automatic retry count, and per-attempt deadlines through explicit finite limits. Structural counters and deterministic fakes SHALL verify those limits without relying on absolute shared-CI timing.

#### Scenario: Reaper queue reaches capacity

- **WHEN** another unconfirmed Shell cleanup is offered after the bounded Reaper queue is full
- **THEN** the system SHALL retain the Shell and runtime ownership in a typed `CloseFailed` state
- **AND** it SHALL reject or defer the new Reaper handoff without dropping handles or starting an unbounded task/thread

#### Scenario: Close worker never reports completion

- **WHEN** a deterministic fake worker never signals completion
- **THEN** the close command SHALL return by its injected deadline
- **AND** the test SHALL observe a bounded number of termination/reap checks and no blocking join

#### Scenario: Startup event burst exceeds its bounded gate

- **WHEN** output generated during `Opening` exceeds the configured startup event buffer or gate capacity
- **THEN** the Shell SHALL enter a typed failure/cleanup path
- **AND** the system SHALL not silently discard output while reporting successful startup
