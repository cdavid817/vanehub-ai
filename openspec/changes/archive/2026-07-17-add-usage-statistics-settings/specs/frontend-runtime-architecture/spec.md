## ADDED Requirements

### Requirement: Usage statistics frontend service boundary
The frontend SHALL expose usage statistics through a service interface and runtime adapters rather than direct runtime calls from React components.

#### Scenario: Desktop usage statistics adapter
- **WHEN** the frontend runs inside the Tauri desktop runtime and usage statistics are requested
- **THEN** the Tauri adapter SHALL call a declared Tauri command through the Agent service boundary

#### Scenario: Web usage statistics adapter
- **WHEN** the frontend runs outside the Tauri desktop runtime and usage statistics are requested
- **THEN** the Web adapter SHALL provide compatible mock aggregation without importing or invoking Tauri APIs

#### Scenario: Usage page service consumption
- **WHEN** the Usage Statistics settings page loads or changes time range
- **THEN** React components SHALL request usage data through the Agent service and shared data-fetching foundation
