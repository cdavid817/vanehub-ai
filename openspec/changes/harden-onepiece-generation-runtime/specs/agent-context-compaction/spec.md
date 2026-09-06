## ADDED Requirements

### Requirement: The synthetic summary is never mistakable for user speech
The system SHALL emit the compaction summary — on the optimizer path and on the compatibility fallback path alike — through one provider-neutral synthetic-summary carrier that is machine-identifiable as synthetic continuation context, mapped per interface format to a non-user or explicitly-marked form. A compaction path SHALL NOT insert an unmarked summary as a bare user turn. Historical transcripts containing marker-style summaries SHALL keep rendering.

#### Scenario: The compatibility path stops emitting bare user turns
- **WHEN** compaction falls back to the compatibility path and produces a summary
- **THEN** the summary SHALL be carried in the identifiable synthetic form
- **AND** SHALL NOT appear as an unmarked `role: "user"` message

#### Scenario: Old transcripts keep rendering
- **WHEN** a session recorded before this change contains a marker-prefixed summary turn
- **THEN** the transcript SHALL continue to render that turn
- **AND** subsequent compactions in that session SHALL use the new carrier
