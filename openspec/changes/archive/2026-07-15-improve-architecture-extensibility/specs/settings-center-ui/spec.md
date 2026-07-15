## ADDED Requirements

### Requirement: Shared settings data orchestration
Settings pages that load or mutate service-backed data SHALL use the shared frontend data-fetching foundation for request state, cache invalidation, refresh, loading, and error behavior.

#### Scenario: Refresh service-backed settings page
- **WHEN** a user refreshes a service-backed settings page
- **THEN** the page SHALL perform the refresh through the shared data-fetching foundation and preserve unrelated local UI state

#### Scenario: Settings mutation succeeds
- **WHEN** a settings page mutation succeeds
- **THEN** the page SHALL invalidate or refresh the affected service-backed data through the shared data-fetching foundation

### Requirement: Shared settings form validation
Settings pages that collect configuration input SHALL use shared schema-backed form validation before submitting through service interfaces.

#### Scenario: Invalid settings form
- **WHEN** a user submits invalid MCP, SDK, provider, Agent, or basic settings input
- **THEN** the settings page SHALL show field-level validation errors and SHALL NOT call a backend or runtime adapter for that invalid submission
