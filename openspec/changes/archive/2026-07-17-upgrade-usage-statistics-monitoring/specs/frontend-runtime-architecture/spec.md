## MODIFIED Requirements

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
