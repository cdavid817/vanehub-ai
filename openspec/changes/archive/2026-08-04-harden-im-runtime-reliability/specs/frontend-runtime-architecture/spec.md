## MODIFIED Requirements

### Requirement: Tauri and Web IM contract parity
The Tauri and Web/mock IM adapters SHALL expose the same method signatures, normalized model shapes, mutation semantics, and lifecycle-status subscription contract.

#### Scenario: Contract conformance test
- **WHEN** frontend contract tests run
- **THEN** they SHALL verify that both adapters implement connector listing, status subscription, routing, configuration, lifecycle, testing, authorization, and binding-reset operations

#### Scenario: Adapter returns normalized mutation state
- **WHEN** routing or connector configuration is saved through either adapter
- **THEN** the adapter SHALL return the normalized state that the React settings surface can use as both editable and persisted state

## ADDED Requirements

### Requirement: Runtime-neutral IM lifecycle updates
The frontend IM service SHALL provide lifecycle updates without exposing Tauri event APIs or platform SDKs to React components.

#### Scenario: Desktop connector lifecycle changes
- **WHEN** a native connector generation changes lifecycle or safe status
- **THEN** the Tauri IM adapter SHALL validate and publish the update through the typed IM service subscription

#### Scenario: Web/mock connector lifecycle changes
- **WHEN** the Web/mock adapter simulates a connector mutation
- **THEN** it SHALL publish deterministic updates through the same typed subscription without persisting plaintext secrets

#### Scenario: Settings page unmounts
- **WHEN** the subscribing React surface unmounts or reloads
- **THEN** it SHALL unsubscribe through the service-provided cleanup handle and SHALL NOT retain duplicate listeners

