## ADDED Requirements

### Requirement: Focused provider-profile creation flow
The Agent Configuration add-profile dialog SHALL separate provider discovery from Agent-specific profile fields with a compact visual hierarchy, SHALL keep the selected provider and current step apparent, and SHALL avoid nested card decoration that competes with the form.

#### Scenario: Open the add-profile dialog
- **WHEN** the user starts adding an Agent provider profile
- **THEN** the dialog SHALL present a searchable provider choice followed by grouped connection, model, and credential fields
- **AND** only fields relevant to the selected Agent and provider SHALL be displayed
- **AND** cancel and save actions SHALL remain visible while the form content scrolls

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
