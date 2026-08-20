## ADDED Requirements

### Requirement: Focused provider-profile creation flow
The Agent Configuration add-profile dialog SHALL separate provider discovery from Agent-specific profile fields into explicit stages, SHALL keep the selected provider and current step apparent, and SHALL avoid nested card decoration that competes with the form.

#### Scenario: Open the add-profile dialog
- **WHEN** the user starts adding an Agent provider profile
- **THEN** the dialog SHALL first present a searchable provider choice and continue to a separate configuration stage
- **AND** the configuration stage SHALL keep required identity, connection, primary-model, and credential fields visible
- **AND** optional provider-specific fields SHALL be placed in a labeled, collapsed advanced section
- **AND** only fields relevant to the selected Agent and provider SHALL be displayed
- **AND** cancel and save actions SHALL remain visible while the form content scrolls

#### Scenario: Edit an existing profile
- **WHEN** the user edits a saved provider profile
- **THEN** the dialog SHALL open directly on the configuration stage without repeating provider selection
- **AND** the saved provider identity SHALL remain visible and unchanged unless the user creates a separate profile

#### Scenario: Use a narrow viewport or keyboard navigation
- **WHEN** the dialog is used at a supported narrow width or entirely from the keyboard
- **THEN** provider selection, fields, validation, and actions SHALL remain reachable without horizontal page overflow
- **AND** focus order and visible focus states SHALL follow the visual flow

### Requirement: Reliable provider identity in profile creation
Every provider preset shown in the add-profile flow SHALL render a stable provider identity using its reviewed icon when available and a readable deterministic fallback when an asset cannot load.

#### Scenario: Render Zhipu GLM
- **WHEN** the Zhipu GLM preset is shown in the provider catalog or selected-provider summary
- **THEN** the Zhipu provider icon SHALL render successfully in light and dark themes
- **AND** the accessible provider name SHALL remain available independently of the image

#### Scenario: Provider asset is unavailable
- **WHEN** a preset icon asset cannot be resolved
- **THEN** the dialog SHALL render the existing deterministic text fallback without a broken-image indicator or missing control geometry

### Requirement: Custom endpoint provider entry
The add-profile provider catalog SHALL include a stable custom endpoint entry for a provider absent from the reviewed preset list, and SHALL validate and save it through the same Agent-specific service boundary as preset-derived profiles.

#### Scenario: Configure an unlisted provider
- **WHEN** the user selects the custom endpoint provider entry
- **THEN** the form SHALL allow a user-defined profile name, validated endpoint, model identifier, and optional credential according to the selected Agent's supported interface
- **AND** no reviewed preset endpoint SHALL be implied or locked

#### Scenario: Save a custom endpoint in Web/mock mode
- **WHEN** a valid custom endpoint profile is saved in Web/mock mode
- **THEN** the adapter SHALL preserve the same profile shape and validation behavior
- **AND** it SHALL NOT claim to write native CLI configuration or a native credential store

### Requirement: Scalable Agent configuration information architecture
The Agent Configuration page SHALL focus on provider/profile management, SHALL use a scalable grouped Agent selector, and SHALL NOT embed workspace-wide code-intelligence administration beneath each selected Agent.

#### Scenario: Select a managed Agent
- **WHEN** the Agent Configuration page is opened or an Agent navigation target is supplied
- **THEN** the page SHALL select the requested Agent in a grouped selector and show only that Agent's provider/profile management content
- **AND** changing the selection SHALL preserve the existing service isolation between Agents
- **AND** LSP configuration, trust, testing, and runtime status SHALL be available from a dedicated Code Intelligence settings page instead of the active Agent panel

#### Scenario: Use the Agent selector responsively
- **WHEN** the number of supported Agents grows or the page is shown at a narrow width
- **THEN** the selector SHALL remain reachable without wrapping into an ambiguous tab grid or causing horizontal page overflow
- **AND** its groups, selected state, keyboard order, and accessible names SHALL remain apparent

#### Scenario: Configure OnePiece capabilities
- **WHEN** OnePiece is selected
- **THEN** provider profiles SHALL be the default secondary view
- **AND** local runtime and tool-readiness controls SHALL be available as explicit secondary views rather than appearing simultaneously in the profile list

### Requirement: Compact profile-management hierarchy
Saved profile presentation SHALL prioritize identity, provider/model, validation, and applied state while placing infrequent details and destructive or duplicative actions behind accessible disclosure controls.

#### Scenario: Browse saved profiles
- **WHEN** the user reviews an Agent's saved profiles
- **THEN** each profile SHALL expose its name, provider, primary model, validation state, and applied state without expanding details
- **AND** endpoint, credential, version, and managed-path details SHALL be available on demand
- **AND** Apply SHALL remain a visible primary action while edit, duplicate, and delete remain keyboard-accessible secondary actions
