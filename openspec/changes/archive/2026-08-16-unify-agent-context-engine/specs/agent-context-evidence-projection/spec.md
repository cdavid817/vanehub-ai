## ADDED Requirements

### Requirement: Context selection manifests are inspectable evidence
The system SHALL project a content-free Context Engine manifest for each completed OnePiece evidence selection through the same desktop and Web/mock service contract, independently of existing compaction evidence cards.

#### Scenario: Selection and compaction both occur
- **WHEN** a turn selects proactive evidence and later triggers compaction
- **THEN** the inspector SHALL distinguish the selection manifest from compaction evidence
- **AND** it SHALL correlate both by stable content-free turn and generation identifiers
