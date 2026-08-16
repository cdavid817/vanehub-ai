## ADDED Requirements

### Requirement: Memory recall participates through an independent context budget
Eligible cross-session memory recall SHALL expose bounded Context Engine candidates and SHALL be ranked and budgeted independently from code evidence while preserving current memory enablement, freshness, deletion, and shared-pool semantics.

#### Scenario: Relevant memory competes with code evidence
- **WHEN** a relevant memory and code candidates are available
- **THEN** memory SHALL consume only its versioned source allocation unless protected by an authoritative rule
- **AND** its body SHALL NOT appear in selection diagnostics or persisted manifest metadata
