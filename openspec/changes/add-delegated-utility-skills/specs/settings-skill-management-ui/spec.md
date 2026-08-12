## ADDED Requirements

### Requirement: Utility delegation presentation
The Skills settings experience SHALL distinguish Utility Skills from Role Skills and present delegation availability, trust, effective revision, declared and effective capabilities, requested and effective limits, assignment support, use count, last use, and unavailable reasons using bounded labels and details.

#### Scenario: Eligible Utility card
- **WHEN** an eligible Utility is shown in inventory
- **THEN** its row SHALL identify it as Utility, delegated rather than injected, and available to supported assigned API Agents

#### Scenario: Utility capability capped
- **WHEN** a declared capability or limit is reduced by platform or permission ceilings
- **THEN** the details SHALL show both declared and effective values with the reason for the cap

#### Scenario: Unsupported CLI selected
- **WHEN** a user selects a CLI Agent without a native delegation adapter
- **THEN** Utility rows SHALL explain that delegated assignment is unavailable and SHALL NOT offer a misleading Assign action

#### Scenario: Invalid Utility metadata
- **WHEN** a Utility is unavailable because of unknown capability, invalid limit, trust, conflict, or effective-content failure
- **THEN** the row SHALL keep preview and repair context visible while disabling delegation assignment

### Requirement: Utility assignment experience
The selected-Agent Skill board SHALL allow assignment and removal of eligible Utilities for native API Agents using stable Agent and canonical Skill ids, while keeping Role injection and CLI mount relationships visually distinct.

#### Scenario: Assign Utility to API Agent
- **WHEN** a user assigns an eligible Utility to a supported native API Agent
- **THEN** the page SHALL call the granular service operation, retain the row until refresh confirms success, and describe the relationship as delegated capability

#### Scenario: Remove Utility assignment
- **WHEN** a user removes an assigned Utility
- **THEN** the operation SHALL not change global enablement, other Agent assignments, Role bindings, active historical attempts, or Overlay content

#### Scenario: Assignment failure
- **WHEN** Utility assignment fails
- **THEN** the row SHALL remain in its prior panel with an actionable row-scoped error while unrelated controls remain operable

### Requirement: Utility delegation history UI
Skill details SHALL provide a bounded paginated delegation-history area showing parent Agent, workspace scope, effective revision, status, start and duration, capability ceiling, tool and approval counts, truncation, and safe result summary without exposing hidden prompts or unrestricted content.

#### Scenario: Load Utility history
- **WHEN** a user opens delegation history for a Utility
- **THEN** the page SHALL load the newest bounded page through the frontend service boundary and support status, Agent, workspace, and time filters

#### Scenario: Inspect attempt
- **WHEN** a user opens a delegation attempt
- **THEN** the detail SHALL link to available execution timeline and approval audit information while respecting observability privacy controls

#### Scenario: No delegation history
- **WHEN** an eligible Utility has never started an approved child attempt
- **THEN** the page SHALL show an explanatory empty state rather than zero-valued fabricated activity

### Requirement: Utility UI runtime parity and accessibility
Desktop and Web/mock Skills settings SHALL use the same frontend service models for Utility eligibility, assignment, capabilities, limits, history, and errors. New controls and dialogs SHALL be keyboard accessible and localized.

#### Scenario: Web Utility details
- **WHEN** Web/mock mode returns an eligible or unavailable Utility
- **THEN** the settings page SHALL render the same capability, assignment, history, and unavailable semantics as equivalent desktop data

#### Scenario: Utility details accessibility
- **WHEN** Utility capability or history details open and close
- **THEN** the UI SHALL expose localized accessible names, manage keyboard focus, support safe dismissal, and restore focus to the trigger

