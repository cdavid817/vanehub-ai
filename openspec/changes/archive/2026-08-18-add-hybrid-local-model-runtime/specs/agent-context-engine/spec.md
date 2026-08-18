## ADDED Requirements

### Requirement: Context planning uses the routed Profile budget
Before collecting or projecting evidence, the Context Engine SHALL consume the immutable Profile selected for that generation and use its effective context window, reserved output, provenance, and confidence. It MUST NOT reuse the globally active Profile when Hybrid Routing selected another Profile.

#### Scenario: Local Profile is selected by a rule
- **WHEN** a Hybrid rule selects a local Profile with a configured conservative context window
- **THEN** context planning SHALL budget against that window and record configured-estimate provenance

#### Scenario: Routed Profile capacity is unknown
- **WHEN** the selected Profile has no verified or configured conservative capacity
- **THEN** the planner SHALL use its existing versioned conservative unknown-capacity ceiling
- **AND** it SHALL not invent utilization or retry indefinitely

#### Scenario: Fallback Profile is selected
- **WHEN** routing chooses a policy-compatible fallback Profile before request construction
- **THEN** context collection and final projection SHALL be recomputed for the fallback Profile budget rather than reusing an oversized projection
