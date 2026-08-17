## MODIFIED Requirements

### Requirement: Model capacity representation
The system SHALL resolve context-window and reserve metadata for the immutable selected endpoint Profile, prefer verified metadata, allow a bounded user-configured conservative value with explicit provenance, and represent unknown capacity without guessing from model identity or inventing utilization.

#### Scenario: Calculate capacity for a known model
- **WHEN** verified context-window metadata exists for the selected Profile and model
- **THEN** the snapshot SHALL expose total capacity, reserved capacity, occupied capacity, remaining capacity, and utilization
- **AND** all derived values SHALL identify the metadata source and versioned policy

#### Scenario: Use a configured conservative capacity
- **WHEN** verified metadata is unavailable and the user configured a valid conservative context window for the selected Profile
- **THEN** the snapshot SHALL use that value with `configured-estimate` provenance
- **AND** it SHALL NOT label the value provider-verified

#### Scenario: Preserve unknown model capacity
- **WHEN** the selected custom model has neither verified nor configured capacity metadata
- **THEN** the snapshot SHALL mark capacity as unknown
- **AND** it SHALL NOT emit a fabricated remaining-token value or utilization percentage

#### Scenario: Same model id appears on another endpoint
- **WHEN** two Profiles share a model id but only one has verified capacity metadata
- **THEN** capacity SHALL remain scoped to the verified Profile and endpoint provenance
- **AND** the other Profile SHALL not inherit it by name
