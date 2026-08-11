## ADDED Requirements

### Requirement: Skill evolution assessment service boundary
The Skill management service SHALL provide workspace- and Skill-scoped assessment summary, detail, history, and policy-status operations through matching desktop/Tauri and Web runtime adapters. React components MUST NOT invoke native commands directly.

#### Scenario: Desktop assessment query
- **WHEN** the desktop UI requests an assessment through the Skill service
- **THEN** the Tauri adapter invokes the native assessment command and returns sanitized typed models

#### Scenario: Web assessment query
- **WHEN** the Web UI requests the same assessment operation
- **THEN** the Web adapter returns behaviorally equivalent mock or backend data with the same status and error semantics

### Requirement: Safe reassessment request
The Skill management service SHALL allow a user to request reassessment of an existing candidate seed without editing evidence, selecting a target manually, changing a Skill, or bypassing quality gates.

#### Scenario: Reassess changed evidence
- **WHEN** the current assessment witness is stale and the user requests reassessment
- **THEN** the service schedules a new immutable attempt and returns its status

#### Scenario: Reassess unchanged evidence
- **WHEN** an identical complete witness already has a current result
- **THEN** the service returns that result without creating duplicate history

### Requirement: Model evaluation policy service
The Skill management service SHALL expose whether optional model evaluation is disabled, unavailable, or enabled and SHALL require an explicit versioned consent update before enabling external evaluation.

#### Scenario: Enable model evaluation
- **WHEN** the user confirms the disclosed sanitized outbound data classes
- **THEN** the service records consent version and enables model consultation for future assessments

#### Scenario: Disable model evaluation
- **WHEN** the user disables model evaluation
- **THEN** subsequent assessments use deterministic behavior without affecting Agent availability or launch state

