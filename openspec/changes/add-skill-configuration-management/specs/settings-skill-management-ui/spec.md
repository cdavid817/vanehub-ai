## ADDED Requirements

### Requirement: Schema-driven Skill configuration panel
The Skill detail surface SHALL render a configuration panel from the normalized backend descriptor with supported text, number, boolean, enum, multi-value, and secret controls; required, advanced, description, default, and validation metadata; and no Skill-specific hard-coded form branches. Components SHALL call the frontend service boundary rather than native commands directly.

#### Scenario: User opens a configurable Skill
- **WHEN** the selected effective Skill has a valid configuration schema
- **THEN** the panel renders ordered controls and current redacted state from the normalized descriptor

#### Scenario: Skill is not configurable
- **WHEN** the selected effective Skill has no configuration schema
- **THEN** the panel shows a non-configurable state without creating an empty stored record

### Requirement: Scope and inheritance visibility
The panel SHALL let users edit User or eligible Project scope and SHALL show each property's local override, inherited source, effective preview, and reset-to-inherited action. Project editing MUST be unavailable without a canonical workspace.

#### Scenario: Project value inherits from User
- **WHEN** the Project scope does not override a property that has a User value
- **THEN** the control identifies User as the effective source without copying the value into Project scope

### Requirement: Safe secret controls
Secret controls SHALL never repopulate stored values. They SHALL display configured/missing/error state and provide explicit replace, preserve, and clear actions with appropriate confirmation and accessible status text.

#### Scenario: User edits another field
- **WHEN** a secret is already configured and the user changes only a non-secret property
- **THEN** the form submits preserve intent and never reads or resends the existing secret

### Requirement: Drift, validation, and stale-write recovery
The panel SHALL show schema-invalid, migration-required, missing-required, validating, valid, save-failed, and stale-editor states. It SHALL preserve unsaved user input on recoverable failure, show field-level and summary errors, and require refresh or explicit reconciliation before overwriting a newer revision.

#### Scenario: Schema changes while editor is open
- **WHEN** the save is rejected because the schema or stored revision changed
- **THEN** the panel retains the draft, identifies the stale state, and offers refresh/reconcile without silently resubmitting

### Requirement: Honest desktop and Web configuration behavior
The Tauri adapter SHALL perform native SQLite and credential-store operations. The Web adapter SHALL implement compatible deterministic non-secret editing for preview/testing but SHALL report secure secret persistence and native runtime consumption as unsupported when no remote backend supplies them; it MUST NOT fabricate configured credentials.

#### Scenario: Web user enters a secret
- **WHEN** no secure remote backend is configured
- **THEN** the Web UI disables secret persistence with an explanation and does not store the value in browser state beyond the active unsaved control

### Requirement: Accessible responsive configuration editing
Configuration controls, scope selection, inheritance indicators, validation summaries, secret actions, and save/reset controls SHALL remain keyboard accessible, screen-reader labeled, and usable without horizontal page overflow on narrow viewports. Status MUST NOT rely on color alone.

#### Scenario: Configuration panel is used on a narrow viewport
- **WHEN** the panel is displayed at the supported narrow breakpoint
- **THEN** labels, controls, provenance, errors, and actions remain readable and operable without horizontal page scrolling

