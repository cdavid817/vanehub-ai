## ADDED Requirements

### Requirement: Plugin integration frontend service boundary
The frontend SHALL expose plugin integration catalog and readiness-test operations through a dedicated service interface and matching runtime adapters rather than direct runtime calls from React components.

#### Scenario: Desktop plugin integration adapter
- **WHEN** the frontend runs inside the Tauri desktop runtime and plugin integrations are listed or tested
- **THEN** the Tauri plugin integration adapter SHALL call declared native commands through the service boundary

#### Scenario: Web plugin integration adapter
- **WHEN** the frontend runs outside the Tauri desktop runtime and plugin integrations are listed or tested
- **THEN** the Web/mock plugin integration adapter SHALL provide deterministic built-in metadata and localized runtime limitation behavior without importing or invoking Tauri APIs

#### Scenario: Plugin integration page service consumption
- **WHEN** the Plugin Integrations settings page loads, refreshes, searches, or tests the GitHub integration
- **THEN** React components SHALL request data through the plugin integration service and shared data-fetching foundation
- **AND** they SHALL preserve already loaded data during an in-progress readiness test

### Requirement: Plugin integration adapter parity
The Tauri and Web/mock plugin integration adapters SHALL expose the same normalized plugin integration contract.

#### Scenario: Contract shape changes
- **WHEN** plugin integration fields are added or changed
- **THEN** shared TypeScript types and adapter tests SHALL verify equivalent field shapes for both runtime implementations
