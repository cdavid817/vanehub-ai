## MODIFIED Requirements

### Requirement: IM routing controls
The IM settings page SHALL provide controls for the default Agent and default project used by new external-chat bindings and SHALL synchronize the form with normalized service results.

#### Scenario: Select routing defaults
- **WHEN** a user edits IM routing settings
- **THEN** the page SHALL use registered Agent ids and service-backed project selection and SHALL show field-level validation before saving

#### Scenario: Save normalized routing defaults
- **WHEN** the routing service accepts and normalizes an Agent id or project path
- **THEN** the page SHALL replace both editable and persisted routing state with the normalized result and SHALL consider the routing ready

#### Scenario: Routing defaults are incomplete
- **WHEN** no valid default Agent or project is configured
- **THEN** the page SHALL explain the incomplete state and SHALL prevent enabling a connector

### Requirement: Safe credential form behavior
The IM settings page SHALL treat secret fields as write-only values and SHALL submit credential edits as field-level patches that preserve omitted stored values.

#### Scenario: Render stored credential
- **WHEN** a connector secret already exists
- **THEN** the page SHALL show a translated configured indicator or redacted placeholder and SHALL NOT receive or render the secret value

#### Scenario: Submit redacted placeholder
- **WHEN** a form contains only the displayed redacted placeholder for a secret field
- **THEN** the page SHALL preserve the existing secret and SHALL NOT submit the placeholder as a replacement value

#### Scenario: Replace credential
- **WHEN** a user enters one new secret or non-secret connector field and saves
- **THEN** the page SHALL send only the edited field through the IM service and SHALL preserve omitted configured fields

#### Scenario: Credential save succeeds
- **WHEN** the native or Web/mock service accepts a credential patch
- **THEN** the page SHALL clear submitted plaintext secret fields from React state and render the normalized non-secret result

#### Scenario: Credential save fails
- **WHEN** the service rejects a credential patch
- **THEN** the page SHALL clear submitted plaintext secret fields, retain safe non-secret edits where useful, and display a localized safe error

## ADDED Requirements

### Requirement: Live connector lifecycle feedback
The IM settings page SHALL keep connector lifecycle summaries current through the runtime-neutral IM service subscription.

#### Scenario: Connector finishes asynchronous startup
- **WHEN** a connector changes from `connecting` to `connected`, `reconnecting`, `authorization-expired`, or `error`
- **THEN** the affected row SHALL apply the newest generation-aware status without requiring manual refresh

#### Scenario: Stale lifecycle update arrives
- **WHEN** the page receives an update from an older connector generation
- **THEN** it SHALL ignore the stale update and preserve the newest known lifecycle and timestamp

