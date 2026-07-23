## ADDED Requirements

### Requirement: Critical frontend interaction verification
The frontend test suite SHALL verify critical user interactions in a DOM environment through user-observable roles, labels, values, and results while keeping runtime calls behind service interfaces.

#### Scenario: Assign a Session by drag and drop
- **WHEN** a user drags an eligible Session card onto a valid category target
- **THEN** the component interaction test SHALL verify that the service-backed category assignment is requested once and the resulting UI state is presented

#### Scenario: Reject an invalid Session drop
- **WHEN** a Session drag ends on an invalid or unavailable category target
- **THEN** the component interaction test SHALL verify that no category mutation is requested and the existing assignment remains presented

#### Scenario: Edit a user Prompt Hook
- **WHEN** a user opens an editable Prompt Hook, changes valid fields, previews it, and saves it
- **THEN** the component interaction test SHALL verify validation, service mutation, related query refresh, and the updated user-visible result

#### Scenario: Prompt Hook save fails
- **WHEN** the Prompt Hook service rejects an edit
- **THEN** the component interaction test SHALL verify that the error is displayed, the entered values remain available, and no direct Tauri API is used

#### Scenario: Built-in Prompt Hook is immutable
- **WHEN** a user inspects a non-editable built-in Prompt Hook
- **THEN** the component interaction test SHALL verify that unsupported mutation controls are unavailable

### Requirement: Runtime-neutral frontend test harness
Frontend interaction tests SHALL use a shared harness for application providers and deterministic service doubles, and the harness MUST remain usable without a Tauri runtime.

#### Scenario: Render a service-backed component test
- **WHEN** a component requires query, localization, theme, or Agent service context
- **THEN** the shared harness SHALL provide deterministic test-owned providers and service responses
- **AND** the component SHALL NOT import or call Tauri `invoke()`

#### Scenario: Simulate frontend service failure
- **WHEN** an interaction test configures a service double to fail
- **THEN** the test SHALL observe the component's public failure behavior without inspecting private hook or component state
