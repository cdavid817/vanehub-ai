## ADDED Requirements

### Requirement: Bounded responsive terminal presentation
The Agent Terminal frontend SHALL batch burst output before rendering and MUST bound retained replay content across all sessions.

#### Scenario: Terminal emits burst output
- **WHEN** multiple output events arrive before the next animation frame
- **THEN** the frontend SHALL coalesce them into a bounded terminal write without changing byte order

#### Scenario: Many sessions retain terminal replay
- **WHEN** retained replay reaches the configured global capacity
- **THEN** the frontend SHALL evict least-recently-used inactive replay entries while preserving the per-session byte bound

