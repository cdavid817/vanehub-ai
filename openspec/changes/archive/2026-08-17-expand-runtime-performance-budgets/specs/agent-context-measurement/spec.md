## ADDED Requirements

### Requirement: Context performance measurements remain content-free
Context performance records SHALL contain only allowlisted phase names, policy and dataset versions, correlations, duration or count buckets, byte and Token estimates, measurement quality, occupancy values, and bounded outcomes. They MUST NOT persist prompt text, message text, tool arguments or results, credentials, request headers, raw provider frames, evidence content, or unrestricted paths.

#### Scenario: Context timing is persisted or exported
- **WHEN** a request snapshot or Context Engine phase emits performance evidence
- **THEN** sensitive context content SHALL be excluded before unified logging or benchmark output
- **AND** occupancy quality SHALL continue to distinguish reported and estimated values

