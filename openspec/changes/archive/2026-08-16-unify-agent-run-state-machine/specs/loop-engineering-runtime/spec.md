## ADDED Requirements

### Requirement: Loop execution projects canonical lifecycle
Each LoopRun SHALL link to a canonical Run and project preparation, acting, verification, retry, pause, stuck, cancellation, and terminal boundaries while retaining Loop phase, limits, no-progress, and human acceptance semantics.

#### Scenario: Loop verifies successfully
- **WHEN** a Loop completes its guarded verification and reaches its existing acceptance boundary
- **THEN** its canonical Run records verification and the owner-defined terminal or blocked outcome without treating acceptance as generic execution
