## ADDED Requirements

### Requirement: Injected evidence has distinct occupancy provenance
Complete request snapshots SHALL measure Context Engine evidence separately from system instructions, declared tools, conversation, and tool-loop additions while retaining compatible aggregate request occupancy and measurement-quality semantics.

#### Scenario: Evidence is projected before provider invocation
- **WHEN** the Context Engine installs a verified evidence set
- **THEN** the next request snapshot SHALL include its evidence occupancy and policy version
- **AND** existing compaction decisions SHALL operate on the complete assembled request
