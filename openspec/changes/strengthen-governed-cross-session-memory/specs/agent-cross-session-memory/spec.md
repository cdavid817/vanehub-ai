## ADDED Requirements

### Requirement: Automatic proposals are applied by immutable id and pinned revision
The trusted layer SHALL resolve every model-referenced memory name from an automatic extraction or body selection against the turn's frozen eligible manifest into an immutable memory id plus the revision the manifest pinned, and SHALL apply updates and archives only through that id and revision. A name matching more than one eligible memory SHALL be rejected as a counted rejection rather than resolved to any match. An update naming no eligible memory SHALL be rejected as a counted rejection and SHALL NOT be converted into a create. Body selection results SHALL be returned as id-and-revision pairs, and an id the manifest never showed SHALL be dropped.

#### Scenario: Duplicate eligible names are rejected, not guessed
- **WHEN** an extraction action names a memory and two eligible memories carry that display name
- **THEN** the system SHALL reject the action as ambiguous and count the rejection
- **AND** SHALL NOT apply the action to either memory

#### Scenario: An unmatched update does not become a create
- **WHEN** an extraction update names a memory that is not in the turn's eligible manifest because it was renamed, archived, or policy-excluded
- **THEN** the system SHALL reject the action as a counted rejection
- **AND** SHALL NOT create a new memory from the update's content

#### Scenario: Body selection returns ids
- **WHEN** the relevance selector answers with selected memories
- **THEN** the system SHALL resolve the selection to id-and-revision pairs from the manifest it was shown
- **AND** SHALL drop any reference the manifest never contained

### Requirement: Turn-end extraction reuses the turn's frozen snapshot
Automatic extraction that runs at the end of a turn SHALL reuse the personalization snapshot and extraction entitlement resolved at that turn's start, and SHALL NOT re-resolve policy mid-turn. A policy edit made while a turn is running SHALL take effect from the next turn only.

#### Scenario: Mid-turn policy edit does not change the running turn's extraction
- **WHEN** a CLI turn starts under a snapshot that permits automatic extraction and the user disables extraction before the turn completes
- **THEN** the turn-end extraction SHALL still be governed by the snapshot resolved at turn start
- **AND** the next turn SHALL resolve a snapshot reflecting the edit

### Requirement: Extraction egress decision and two-phase secret gate
Before any provider call carries conversation content for automatic extraction, the system SHALL evaluate an explicit egress decision bound to a declared extraction profile, and SHALL apply secret redaction to the extraction input. Secret redaction SHALL run again before a produced candidate is persisted. A refused egress decision SHALL skip extraction without failing the delivered turn.

#### Scenario: CLI content does not silently leave through another provider
- **WHEN** a CLI turn's content would be extracted through a provider profile different from the one that produced the conversation
- **THEN** the system SHALL evaluate the egress decision for that extraction profile before sending any content
- **AND** a refusal SHALL be logged as a skipped extraction while the delivered CLI response is unaffected

#### Scenario: Secrets are gated twice
- **WHEN** extraction input contains a detectable secret
- **THEN** the system SHALL redact or refuse before the provider call
- **AND** SHALL redact or refuse again before persisting any resulting candidate

### Requirement: Automatic extraction records producer, trigger, and extractor separately
An automatically produced candidate SHALL record the producing Agent as its source (a CLI turn's candidate records the CLI Agent, not the extracting provider's Agent), the trigger kind (compaction, turn end, episode terminal, or explicit tool), and the extraction profile that performed the model call, as three separate facts.

#### Scenario: CLI-produced candidates are attributed to the CLI Agent
- **WHEN** OnePiece's provider performs extraction on behalf of a completed CLI turn
- **THEN** the resulting candidate's source SHALL identify the CLI Agent as producer
- **AND** the extraction profile SHALL be recorded separately from the producer
