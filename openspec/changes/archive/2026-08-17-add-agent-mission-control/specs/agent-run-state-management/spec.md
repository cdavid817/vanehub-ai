## ADDED Requirements

### Requirement: Mission Control Run projection and retry control
The shared Run service SHALL expose a bounded Mission Control projection over canonical Run state and a retry control that delegates eligibility and execution to the owning runtime. The projection MUST preserve canonical Run identity and terminal semantics and MUST NOT become a second lifecycle authority.

#### Scenario: Projection is queried
- **WHEN** Mission Control requests a filtered page
- **THEN** the shared service returns bounded summaries derived from canonical state using contract-compatible Tauri and Web/mock adapters

#### Scenario: Retry is accepted
- **WHEN** an eligible failed or stuck Run is retried
- **THEN** the owning runtime creates or transitions work according to its existing retry policy and returns the resulting canonical Run identity and state

#### Scenario: Retry is rejected
- **WHEN** state, owner policy, permission, or version does not allow retry
- **THEN** the service returns a safe typed rejection and does not alter the canonical Run
