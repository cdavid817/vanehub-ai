## ADDED Requirements

### Requirement: Metadata-only review lifecycle logging
The unified log service SHALL record review creation, bounded diff load, comment state changes, stale detection, revert outcomes, feedback receipts, and automated action outcomes using redacted metadata only.

#### Scenario: Persist review diagnostic
- **WHEN** a review lifecycle event is persisted
- **THEN** it SHALL contain only safe ids, relative-path fingerprints, counts, sizes, timing, operation ids, and outcome categories

#### Scenario: Reject sensitive review fields
- **WHEN** event input includes code, diff text, comment/finding bodies, prompts, secrets, absolute paths, or raw tool output
- **THEN** those fields SHALL be excluded or redacted before disk persistence
