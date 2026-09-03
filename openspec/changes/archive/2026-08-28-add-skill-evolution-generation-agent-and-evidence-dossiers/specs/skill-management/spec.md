## ADDED Requirements

### Requirement: Skill generation service boundary
The Skill management service SHALL expose generation consent, request, job, stage, dossier, export, attempt, draft, validation, cancellation, regeneration, quarantine, and Curator-handoff operations through matching desktop/Tauri and Web adapters. React components MUST NOT invoke native commands directly.

#### Scenario: Desktop generation request
- **WHEN** the desktop UI requests generation through the Skill service
- **THEN** the Tauri adapter invokes the native generation boundary and returns typed job status

#### Scenario: Web generation request
- **WHEN** Web/mock UI submits the same operation
- **THEN** the Web adapter returns equivalent simulated stages and explicit mock provenance without calling a local provider

### Requirement: Conflict-safe generated Skill creation
Installation of an approved quarantined new Skill SHALL use the existing Skill creation transaction with expected proposal, workspace, catalog, scope, and candidate-id witnesses. It SHALL reject collisions and stale state without partially creating files, registry rows, or bindings.

#### Scenario: Approved proposal creates a Skill
- **WHEN** current Curator approval and all creation witnesses are valid
- **THEN** the service creates one User or Project Skill with generation provenance and returns its canonical identity

#### Scenario: Creation transaction fails
- **WHEN** the source directory or registry commit cannot complete atomically
- **THEN** the system restores the prior catalog state and leaves the proposal unapplied

### Requirement: Bounded generation payloads
Generation service responses and exports SHALL be sanitized, size bounded, paginated where needed, and explicit about truncation and completeness. They MUST exclude raw model prompts, provider payloads, unsafe rejected content, and prohibited source data.

#### Scenario: Dossier response is large
- **WHEN** a dossier exceeds one response page
- **THEN** the service returns stable section pagination without silently omitting evidence completeness markers

