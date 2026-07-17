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

### Requirement: Common settings adapter boundary
The frontend SHALL expose common settings operations through the service interface and runtime adapters rather than direct runtime calls from React components.

#### Scenario: Desktop common settings adapter
- **WHEN** the frontend runs inside the Tauri desktop runtime and common settings are loaded, saved, or inspected for Node.js information
- **THEN** the Tauri adapter SHALL call declared Tauri commands through the service boundary

#### Scenario: Web common settings adapter
- **WHEN** the frontend runs outside the Tauri desktop runtime and common settings are loaded, saved, or inspected for Node.js information
- **THEN** the Web adapter SHALL provide Web-compatible behavior without importing or invoking Tauri APIs

#### Scenario: Components use settings provider
- **WHEN** React components render or mutate common settings
- **THEN** they SHALL use the settings provider or frontend service interface instead of calling runtime-specific APIs directly

### Requirement: Global preference application
The frontend SHALL apply language, font-size, and theme settings at application scope in a runtime-independent way.

#### Scenario: Apply language globally
- **WHEN** the settings provider receives a valid language setting
- **THEN** it SHALL update i18next so all localized components use the selected language

#### Scenario: Apply root font size globally
- **WHEN** the settings provider receives a valid font size setting
- **THEN** it SHALL update the root document font size and SHALL NOT use CSS `zoom` for global scaling

#### Scenario: Apply theme globally
- **WHEN** the settings provider receives a valid visual theme setting
- **THEN** it SHALL update the document theme attribute used by shared CSS variables

### Requirement: Application-wide frontend localization
The frontend SHALL render user-visible application text through synchronized Simplified Chinese and English i18n resources.

#### Scenario: Render active language across frontend surfaces
- **WHEN** the application language is set to Simplified Chinese or English
- **THEN** page titles, descriptions, actions, placeholders, status labels, notices, confirmations, modal labels, empty states, and user-facing frontend errors SHALL render from the active locale resource
- **AND** React page components SHALL NOT hard-code those user-visible strings outside the i18n resources

#### Scenario: Preserve stable identifiers
- **WHEN** the UI displays provider names, Agent display names, package names, executable names, file paths, command strings, protocol names, model names, or stable ids
- **THEN** the frontend MAY render those identifiers literally without translating them

#### Scenario: Keep locale resources aligned
- **WHEN** a frontend translation key is added, removed, or renamed
- **THEN** the zh-CN and en locale resources SHALL contain the same key set
- **AND** automated tests SHALL fail when the key sets diverge

#### Scenario: Format dates with active language
- **WHEN** frontend code formats user-visible dates or times
- **THEN** it SHALL use the active i18n language or an explicit locale derived from it rather than a hard-coded locale unrelated to the active language

### Requirement: Project i18n development contract
The project standards SHALL require all future frontend page changes to support Simplified Chinese and English localization.

#### Scenario: Add or change user-visible page text
- **WHEN** a developer adds or changes user-visible text in a React page, shared UI module, dialog, or frontend-owned service message
- **THEN** the change SHALL add or update both zh-CN and en translation values
- **AND** the implementation SHALL keep translation parity and hard-coded text guardrail tests passing

#### Scenario: Document i18n standard
- **WHEN** project standards are updated for frontend development rules
- **THEN** they SHALL state that new page/UI changes must use i18n resources for both zh-CN and en rather than hard-coded user-visible copy

### Requirement: Service-backed CLI refresh state
The frontend SHALL represent all-tool and single-tool CLI refresh loading, running, success, and failure states through service-backed data and operation status.

#### Scenario: Refresh state uses service boundary
- **WHEN** the CLI management settings page starts or observes a CLI refresh operation
- **THEN** React components SHALL use the Agent service and operation service interfaces to derive page or card refresh state
- **AND** React components SHALL NOT import or call Tauri APIs directly

#### Scenario: Targeted refresh preserves unrelated state
- **WHEN** a targeted refresh runs for one stable agent id
- **THEN** the frontend SHALL preserve cached status and interaction for unrelated CLI cards

#### Scenario: Web runtime simulates refresh state
- **WHEN** the CLI management settings page runs in the Web/mock runtime and a refresh is requested
- **THEN** the Web adapter SHALL return a mock operation that allows the page to show refresh-in-progress behavior without requiring native commands or writing local log files

### Requirement: Detailed CLI environment adapter parity
The Tauri and Web/mock Agent service adapters SHALL implement the same normalized detailed CLI environment contract.

#### Scenario: Desktop adapter returns native status
- **WHEN** the desktop frontend requests CLI status
- **THEN** only the Tauri adapter SHALL invoke native commands and SHALL return cached installation distribution, active entry, source, environment, health, conflict, and lifecycle eligibility fields

#### Scenario: Web adapter remains honest
- **WHEN** the Web/mock frontend requests CLI status
- **THEN** the Web adapter SHALL return the fixed supported catalog with unsupported native detection and empty installation distribution rather than fake host paths or versions

#### Scenario: Contract shape changes
- **WHEN** detailed CLI environment fields are added or changed
- **THEN** shared contract conformance and adapter tests SHALL verify equivalent field shapes for both runtime implementations

### Requirement: Frontend critical CLI failure reporting
The frontend SHALL report critical CLI refresh and package-operation failures through the service boundary when those failures require durable diagnostics beyond the operation log.

#### Scenario: Report refresh start failure
- **WHEN** starting a CLI refresh request fails before the backend returns an operation id
- **THEN** the frontend SHALL surface a user-displayable error
- **AND** in the Tauri runtime it SHALL report the failure through the logging service boundary for native persistence

#### Scenario: Report package start failure
- **WHEN** starting a CLI package operation fails before the backend returns an operation id
- **THEN** the frontend SHALL surface a user-displayable error
- **AND** in the Tauri runtime it SHALL report the failure through the logging service boundary for native persistence

### Requirement: Responsive long-running service operations
The frontend SHALL handle potentially long-running refresh, download, network, package, connection-test, filesystem-backed, and native task operations through service interfaces that expose observable asynchronous state without blocking React rendering.

#### Scenario: Start long-running service operation
- **WHEN** a React surface starts an operation that may perform refresh, download, remote resource access, package management, external command execution, connection testing, filesystem scanning, Git work, or database-heavy native work
- **THEN** the React surface SHALL call a frontend service interface that returns or observes asynchronous operation state
- **AND** the React surface SHALL NOT call Tauri `invoke()` directly

#### Scenario: Preserve loaded data during refresh
- **WHEN** a service-backed page refreshes data while prior data is already available
- **THEN** the page SHALL keep the prior data visible as stale or refreshing state instead of replacing the surface with a blocking blank state

#### Scenario: Show terminal operation result
- **WHEN** a long-running service operation completes, partially completes, or fails
- **THEN** the frontend SHALL represent the terminal status and user-displayable result or error through the service-backed state model

### Requirement: Runtime adapter parity for async operations
Runtime-specific frontend adapters SHALL expose the same asynchronous operation contracts for desktop and Web runtimes.

#### Scenario: Desktop adapter starts async native operation
- **WHEN** the frontend runs in the Tauri desktop runtime and a long-running operation is requested
- **THEN** the Tauri adapter SHALL call a declared Tauri command through the service boundary and return the backend operation or task identity without requiring React components to know native details

#### Scenario: Web adapter simulates async operation
- **WHEN** the frontend runs in browser Web runtime and a long-running operation is requested
- **THEN** the Web adapter SHALL provide compatible mock or future HTTP-backed asynchronous state so loading, running, success, and failure UI behavior remains testable

### Requirement: Project async operation development contract
The project standards SHALL require future frontend and adapter changes to treat potentially time-consuming operations as asynchronous work with observable state.

#### Scenario: Document async operation standard
- **WHEN** project standards are updated for performance and responsiveness rules
- **THEN** they SHALL state that refresh, download, network resource access, package operations, external command execution, connection testing, filesystem scanning, Git operations, and database-heavy work must not block frontend rendering or the desktop shell

#### Scenario: Add new time-consuming frontend behavior
- **WHEN** a developer adds a frontend workflow that triggers potentially long-running work
- **THEN** the workflow SHALL expose loading or running state, preserve relevant already loaded data where possible, and route the work through the service and runtime adapter boundary

### Requirement: Usage statistics frontend service boundary
The frontend SHALL expose separated usage monitoring data through the Agent service interface and matching runtime adapters rather than direct runtime calls from React components.

#### Scenario: Desktop usage statistics adapter
- **WHEN** the frontend runs inside the Tauri desktop runtime and usage statistics are requested
- **THEN** the Tauri adapter SHALL call a declared Tauri command through the Agent service boundary
- **AND** it SHALL return reported-token totals, estimated-character totals, coverage, daily trends, and stable-Agent-id breakdowns using the shared frontend contract

#### Scenario: Web usage statistics adapter
- **WHEN** the frontend runs outside the Tauri desktop runtime and usage statistics are requested
- **THEN** the Web adapter SHALL provide compatible mock usage records and aggregation without importing or invoking Tauri APIs
- **AND** it SHALL preserve the same accounting-quality and local-calendar semantics as the desktop contract

#### Scenario: Usage page service consumption
- **WHEN** the Usage Statistics settings page loads, changes time range, polls while mounted, or refreshes manually
- **THEN** React components SHALL request usage data through the Agent service and shared data-fetching foundation
- **AND** they SHALL preserve already loaded data during an in-progress refresh

#### Scenario: Runtime contract parity
- **WHEN** the Tauri and Web adapters return the same normalized usage fixtures for the same range
- **THEN** their separated totals, coverage counts, trend bucket dates, and Agent attribution SHALL be equivalent

### Requirement: CLI parameter frontend service contract
The frontend agent service SHALL expose runtime-neutral methods to list, save, and reset typed CLI parameter profiles.

#### Scenario: React loads parameter profiles
- **WHEN** the CLI Parameter Management page needs parameter definitions or selections
- **THEN** the React component SHALL call the frontend agent service interface
- **AND** it SHALL NOT call Tauri `invoke()`, inspect SQLite, or branch on the active runtime

#### Scenario: React mutates parameter profile
- **WHEN** the user saves or resets a CLI profile
- **THEN** the React component SHALL submit typed values through the frontend agent service
- **AND** it SHALL render the normalized profile returned by the service

### Requirement: CLI parameter adapter parity
The Tauri and Web/mock agent adapters SHALL implement the same typed CLI parameter profile contract and validation-visible response shapes.

#### Scenario: Desktop adapter operation
- **WHEN** the frontend runs in the Tauri desktop runtime
- **THEN** only the Tauri-specific adapter SHALL invoke the native list, save, or reset command

#### Scenario: Web adapter operation
- **WHEN** the frontend runs in Web/mock mode
- **THEN** the Web adapter SHALL provide catalog, persistence, reset, validation, and preview behavior without requiring native commands
- **AND** local CLI process launch SHALL remain unavailable in Web/mock mode

### Requirement: CLI parameter contract conformance
Frontend contracts and adapter fixtures SHALL preserve stable agent ids, parameter ids, control kinds, value shapes, and default semantics across runtime implementations.

#### Scenario: Run adapter contract tests
- **WHEN** frontend contract conformance tests execute
- **THEN** both adapters SHALL satisfy the same profile shape and mutation semantics for the four stable CLI agent ids

