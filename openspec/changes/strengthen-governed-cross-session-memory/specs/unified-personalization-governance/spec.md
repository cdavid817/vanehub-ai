## ADDED Requirements

### Requirement: Candidate application is idempotent end to end
A create candidate SHALL reserve its resulting memory id at submission time. Approval SHALL record an apply operation in a durable ledger before mutating the authoritative store, then apply, then mark the candidate reviewed. Recovery after an interruption at any point SHALL complete or roll the operation forward such that retrying an approval yields exactly the record the reserved id names, and SHALL NOT create a second memory from one proposal.

#### Scenario: Crash between apply and mark-reviewed does not duplicate
- **WHEN** approval has written the authoritative file for a create candidate and the process crashes before the candidate is marked reviewed
- **THEN** startup recovery or a retried approval SHALL converge on the single memory identified by the reserved resulting id
- **AND** the candidate SHALL end marked approved exactly once

#### Scenario: Reserved id is stable across retries
- **WHEN** an approval is retried after a transient persistence failure
- **THEN** every attempt SHALL target the same reserved resulting memory id
- **AND** no attempt SHALL allocate a fresh id for the same candidate
