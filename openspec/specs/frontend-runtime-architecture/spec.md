# frontend-runtime-architecture Specification

## Purpose
Defines the frontend routing, runtime adapter, data-fetching, validation, error isolation, and workspace modularity foundation for scalable Tauri desktop and browser Web runtimes.

## Requirements
### Requirement: Routed frontend surfaces
The frontend SHALL expose top-level application surfaces through a routing layer that can address workspace, settings, and future detail views without relying on a single component-local view flag.

#### Scenario: Navigate to settings
- **WHEN** a user opens the settings route in the Tauri desktop runtime or browser Web runtime
- **THEN** the frontend SHALL render the settings center through the route while preserving runtime adapter boundaries

#### Scenario: Navigate to workspace
- **WHEN** a user opens the workspace route in the Tauri desktop runtime or browser Web runtime
- **THEN** the frontend SHALL render the workspace surface through the route without requiring a Tauri-only backend call

### Requirement: Shared data fetching foundation
The frontend SHALL use a shared server-state fetching and mutation foundation for service-backed Agent, MCP, SDK, settings, and future workspace data.

#### Scenario: Load service-backed data
- **WHEN** a service-backed page loads data
- **THEN** the page SHALL use the shared fetching foundation to represent loading, success, error, retry, and refresh states

#### Scenario: Mutate service-backed data
- **WHEN** a service-backed page creates, updates, deletes, tests, installs, or launches data through a service interface
- **THEN** the page SHALL invalidate or refresh related cached state through the shared fetching foundation

### Requirement: Runtime adapter selection
The frontend SHALL resolve runtime-specific service adapters through an explicit runtime adapter factory that supports Tauri desktop, browser Web mock, and future HTTP-backed Web adapters.

#### Scenario: Desktop runtime selected
- **WHEN** the frontend runs inside the Tauri desktop runtime
- **THEN** the runtime factory SHALL provide adapters that call Tauri-specific service implementations

#### Scenario: Web runtime selected
- **WHEN** the frontend runs outside the Tauri desktop runtime
- **THEN** the runtime factory SHALL provide Web-compatible adapters that do not require native Tauri commands

### Requirement: Frontend error isolation
The frontend SHALL isolate rendering failures and async service failures at route or feature boundaries so a failing page does not break the entire application shell.

#### Scenario: Feature render failure
- **WHEN** a feature panel throws during render
- **THEN** the frontend SHALL show a bounded error fallback for that feature or route while preserving the surrounding shell

#### Scenario: Service failure
- **WHEN** a service request fails
- **THEN** the frontend SHALL show a user-displayable error derived from the service error without losing already loaded unrelated page state

### Requirement: Form validation foundation
The frontend SHALL use shared schema-backed form validation for configuration forms that submit to Agent, MCP, SDK, provider, or settings services.

#### Scenario: Invalid form submission
- **WHEN** a user submits a configuration form with invalid values
- **THEN** the frontend SHALL prevent submission and show field-level validation errors before calling the service interface

#### Scenario: Backend validation failure
- **WHEN** the backend rejects a submitted configuration
- **THEN** the frontend SHALL display the backend validation error in the form or page error area without bypassing the service interface

### Requirement: Modular workspace shell
The workspace UI SHALL be split into service-backed modules for navigation, conversation/workflow content, agent graph or chat views, composer controls, and runtime details.

#### Scenario: Replace prototype workspace data
- **WHEN** real workspace data services become available
- **THEN** the workspace modules SHALL load data through service interfaces instead of hard-coded arrays in the layout component

#### Scenario: Preserve Web preview behavior
- **WHEN** the workspace runs in browser Web runtime before real backend services exist
- **THEN** the workspace SHALL use Web adapter data rather than importing desktop-only runtime code
