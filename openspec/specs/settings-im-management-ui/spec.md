# settings-im-management-ui Specification

## Purpose
TBD - created by archiving change split-settings-center-ui-spec. Update Purpose after archive.
## Requirements
### Requirement: Service-backed IM settings page
The settings center SHALL include a localized IM entry and service-backed page before Usage Statistics and About.

#### Scenario: Navigate to IM settings
- **WHEN** the settings navigation renders
- **THEN** it SHALL show an icon-backed IM entry that opens the IM management page without a full-page reload

#### Scenario: Load IM settings
- **WHEN** the IM page opens
- **THEN** it SHALL load connector descriptors, current status, credential-presence metadata, and routing settings through the frontend IM service

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

### Requirement: Connector management rows
The IM settings page SHALL render one expandable management row for each of the five built-in connectors.

#### Scenario: Display connector summary
- **WHEN** a connector row is collapsed
- **THEN** it SHALL show platform identity, stable status, experimental marker when applicable, enablement, and the most recent safe status timestamp

#### Scenario: Expand connector configuration
- **WHEN** a user expands a connector row
- **THEN** it SHALL show required non-secret and secret fields, connection actions, official documentation action, and concise status feedback without nesting cards inside cards

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

### Requirement: Connector actions and authorization UX
The IM settings page SHALL expose enable, disable, test, retry, clear, and platform-specific authorization actions according to connector state.

#### Scenario: Test connection
- **WHEN** the user activates a supported connector test
- **THEN** only the affected row SHALL enter a busy state and display the localized result

#### Scenario: Start personal WeChat QR authorization
- **WHEN** the user starts personal WeChat authorization
- **THEN** the page SHALL render the short-lived QR result in a focused authorization surface with cancel, expiry, and retry states

#### Scenario: Connector reports authentication expiry
- **WHEN** connector status becomes `authorization-expired`
- **THEN** the page SHALL show a localized reauthorization action rather than an indefinite connecting state

### Requirement: IM settings visual parity
The IM settings page SHALL use shared settings primitives and semantic tokens in both registered visual styles.

#### Scenario: Render futuristic style
- **WHEN** the IM settings page renders with `futuristic`
- **THEN** connector rows, controls, status tones, dialogs, and focus states SHALL remain readable within the dark operational style

#### Scenario: Render minimal style
- **WHEN** the IM settings page renders with `minimal`
- **THEN** the same controls and states SHALL remain readable in the bright compact style without dark-only assumptions

#### Scenario: Responsive settings layout
- **WHEN** the settings content width is reduced
- **THEN** labels, status controls, credentials, and actions SHALL wrap or reflow without overlap, clipping, or horizontal layout shifts

### Requirement: IM settings localization
All IM settings page text, connector setup copy owned by VaneHub, actions, statuses, validation, accessible names, notices, and errors SHALL use synchronized zh-CN and en resources.

#### Scenario: Verify locale parity
- **WHEN** automated frontend tests inspect the translation resources
- **THEN** every IM settings translation key SHALL exist with equivalent meaning in both zh-CN and en

### Requirement: Live connector lifecycle feedback
The IM settings page SHALL keep connector lifecycle summaries current through the runtime-neutral IM service subscription.

#### Scenario: Connector finishes asynchronous startup
- **WHEN** a connector changes from `connecting` to `connected`, `reconnecting`, `authorization-expired`, or `error`
- **THEN** the affected row SHALL apply the newest generation-aware status without requiring manual refresh

#### Scenario: Stale lifecycle update arrives
- **WHEN** the page receives an update from an older connector generation
- **THEN** it SHALL ignore the stale update and preserve the newest known lifecycle and timestamp

