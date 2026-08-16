## ADDED Requirements

### Requirement: Approval waits project canonical state
A pending approval for executing work SHALL transition its canonical Run to waiting approval and an allow, deny, expiry, generation end, or cancellation decision SHALL leave that state through the guarded transition contract.

#### Scenario: Late approval follows cancellation
- **WHEN** an approval arrives after its Run was cancelled
- **THEN** it is rejected and cannot resume or execute the cancelled work
