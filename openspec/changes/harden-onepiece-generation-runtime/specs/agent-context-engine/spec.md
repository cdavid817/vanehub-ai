## ADDED Requirements

### Requirement: The context budget derives from the active model's capacity
The system SHALL derive the context-engine budget for a generation from the resolved capacity of the snapshot's endpoint and model, with source priority explicit profile override, then the model context catalog, then a conservative default, and SHALL NOT keep a fixed runtime budget that ignores the active model. Reserves SHALL scale with the resolved capacity rather than being fixed absolute values. When no capacity metadata exists for the model, the system SHALL fall back to character-based accounting and surface the degraded measurement quality.

#### Scenario: A large-context model gets a larger evidence budget
- **WHEN** the active profile resolves to a model whose catalog capacity exceeds the previous fixed budget
- **THEN** evidence selection SHALL operate within a budget derived from that capacity
- **AND** SHALL NOT be clipped to a fixed constant that ignores the model

#### Scenario: Missing capacity metadata degrades visibly
- **WHEN** the active model has no capacity entry in any source
- **THEN** the system SHALL use the character-based fallback
- **AND** SHALL record the degraded measurement quality rather than fabricating a capacity

### Requirement: Retrieved evidence never rides inside the user's message
The system SHALL deliver retrieved context evidence to the provider in a typed envelope rendered as a clearly-attributed non-user section, carrying provenance and redaction state per item, and SHALL NOT append evidence into the user's message text. The user's own words SHALL reach the provider verbatim.

#### Scenario: The user message stays verbatim
- **WHEN** evidence is selected for a generation
- **THEN** the user message sent to the provider SHALL contain exactly what the user wrote
- **AND** the evidence SHALL travel in its own attributed section

#### Scenario: Evidence carries provenance
- **WHEN** an evidence item enters the envelope
- **THEN** it SHALL carry its source kind, source identity, and redaction state
- **AND** the context-evidence manifest SHALL record what the envelope contained
