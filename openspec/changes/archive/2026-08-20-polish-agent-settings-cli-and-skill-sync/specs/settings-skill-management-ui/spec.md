## ADDED Requirements

### Requirement: Skill drift banner reflects post-synchronization state
The Skill Management page SHALL replace its active drift report with a freshly loaded post-synchronization overview and SHALL distinguish completed repairs from remaining failures.

#### Scenario: Synchronization resolves all visible drift
- **WHEN** synchronization succeeds and the refreshed overview contains no drift issues
- **THEN** the page SHALL show the synchronized state and repair summary
- **AND** it SHALL not continue to show the previous issue count or enable another no-op synchronization

#### Scenario: Synchronization leaves failures
- **WHEN** the synchronization result or refreshed overview contains unresolved issues
- **THEN** the page SHALL show the remaining issue count and bounded per-Skill failure information
- **AND** it SHALL not describe failed items as synchronized
