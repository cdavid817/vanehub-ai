## ADDED Requirements

### Requirement: Evolution assessment summary
The Skill Evolution area SHALL present the current target-selection classification, leading target and alternatives, nine-check quality summary, confidence, risk, routing recommendation, and whether the result is deterministic, model-assisted, or a fallback.

#### Scenario: Clear deterministic assessment
- **WHEN** an assessment has a clear target and deterministic result
- **THEN** the UI shows the selected effective revision, score components, threshold margin, check outcomes, and routing explanation

#### Scenario: Ambiguous assessment
- **WHEN** target selection remains ambiguous
- **THEN** the UI shows ranked alternatives and uncertainty without presenting any target as causally proven

#### Scenario: No assessment exists
- **WHEN** evidence is unready, assessment is pending, or no result exists
- **THEN** the UI shows the corresponding non-destructive state rather than fabricated assessment data

### Requirement: Assessment detail and history
The Skill Evolution area SHALL let users inspect safe evidence references, target score explanations, all nine check results, model or fallback provenance, version witnesses, and superseded assessment history.

#### Scenario: Inspect a failed quality check
- **WHEN** the user opens a check that blocked advancement
- **THEN** the UI shows its stable reason, sanitized supporting references, and effect on routing

#### Scenario: Inspect superseded attempt
- **WHEN** a newer assessment replaced an older result
- **THEN** the UI identifies which evidence or policy witness changed and keeps both attempts read-only

### Requirement: Model evaluation consent controls
The Skill Evolution area SHALL keep model evaluation disabled by default and SHALL disclose sanitized outbound data categories, provider availability, deterministic fallback behavior, and revocation effects before accepting consent.

#### Scenario: User enables evaluation
- **WHEN** the user explicitly confirms the current disclosure
- **THEN** the UI updates policy through the Skill service and displays the consent version and enabled state

#### Scenario: Provider is unavailable
- **WHEN** no compatible configured model is available
- **THEN** the UI keeps deterministic assessment usable and explains that optional consultation is unavailable

### Requirement: Safe reassessment control
The Skill Evolution area SHALL expose reassessment only for existing candidate seeds and SHALL explain that it creates an audit attempt rather than modifying evidence or Skills.

#### Scenario: User requests reassessment
- **WHEN** the user activates reassessment
- **THEN** the UI submits through the Skill service, shows queued or current status, and preserves the previous result during processing

#### Scenario: Reassessment fails
- **WHEN** scheduling or assessment fails
- **THEN** the UI retains the last valid result and presents an actionable retry message

### Requirement: Assessment UI remains non-mutating
The assessment UI SHALL NOT expose approve, reject, target override, Overlay edit, apply, unpin, archive, memory-write, or automatic-evolution actions in this change.

#### Scenario: Recommendation is advance
- **WHEN** an assessment recommends `advance`
- **THEN** the UI describes it as ready for a later governance stage and provides no mutation action

