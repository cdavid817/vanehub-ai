## ADDED Requirements

### Requirement: Delegated group execution uses child Runs
Multi-Agent delegated execution SHALL use parent/child canonical Run links while Seat assignment, turn ownership, speaker identity, and human routing remain owned by group chat.

#### Scenario: Delegated turn is cancelled by parent
- **WHEN** the parent generation is cancelled during a delegated turn
- **THEN** the child Run is cancelled without changing persisted Seat or speaker semantics
