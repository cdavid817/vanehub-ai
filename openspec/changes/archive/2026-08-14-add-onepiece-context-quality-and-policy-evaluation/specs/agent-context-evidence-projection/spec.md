## ADDED Requirements

### Requirement: Evidence cards correlate with quality assessments
Every successful compaction evidence card SHALL expose the same content-free attempt correlation used by its quality assessment so restored chat evidence can be reconciled with bounded policy-health history.

#### Scenario: Assessment persistence succeeds
- **WHEN** successful compaction produces both a rich evidence card and a persisted assessment
- **THEN** both projections SHALL carry the same stable attempt correlation and policy version

#### Scenario: Assessment persistence fails
- **WHEN** successful compaction evidence is produced but assessment persistence is unavailable
- **THEN** the evidence card SHALL remain available with its attempt correlation and generation SHALL remain successful

