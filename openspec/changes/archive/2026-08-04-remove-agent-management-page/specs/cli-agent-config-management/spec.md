## MODIFIED Requirements

### Requirement: Dedicated Agent configuration management page
The settings experience SHALL provide one dedicated, lazy-loaded Agent Configuration page for OnePiece and CLI provider configuration that remains visually and behaviorally separate from runtime Agent selection and registered-Agent management.

#### Scenario: Navigate from Agent management
- **WHEN** the user chooses to manage global configuration from the Agents page or a supported Agent card
- **THEN** settings SHALL open the dedicated Agent Configuration page
- **AND** MAY preselect the originating stable Agent id without changing the selected Session, runtime Agent, or workflow

#### Scenario: Open the dedicated Agent Configuration page
- **WHEN** the user selects Agent Configuration in settings or follows a supported Agent configuration link
- **THEN** settings SHALL open the Agent Configuration page
- **AND** MAY preselect the originating configuration Agent id without changing the selected Session, runtime Agent, or workflow
- **AND** SHALL NOT expose a separate Agent Management page or registered-Agent management controls

#### Scenario: Switch configuration Agent
- **WHEN** the user selects the OnePiece, Claude Code, OpenCode, or Codex tab
- **THEN** the page SHALL show that Agent's provider configuration controls through the frontend service boundary
- **AND** a CLI Agent tab SHALL retain its compact status strip, focused add/optional-import/refresh/search toolbar, and saved profile list
- **AND** switching configuration tabs SHALL NOT invoke runtime Agent selection

#### Scenario: Review startup synchronization outcome
- **WHEN** startup synchronization imports, updates, skips, or cannot parse local configuration
- **THEN** the page SHALL expose a compact secret-free outcome or warning without requiring candidate selection
- **AND** SHALL keep saved-profile management usable

#### Scenario: Review saved provider profiles
- **WHEN** the selected Agent has saved profiles
- **THEN** each profile card SHALL show its provider identity, profile name, endpoint, primary or default model, credential presence, validation state, and available lifecycle actions
- **AND** the globally applied profile SHALL have persistent visual emphasis and an explicit applied label that does not depend on hover

#### Scenario: Search saved provider profiles
- **WHEN** the user searches the selected Agent's saved profiles
- **THEN** the page SHALL filter the primary profile list without exposing or searching credential values
- **AND** SHALL show a distinct filtered-empty state when no profile matches

#### Scenario: Discover a common provider while creating
- **WHEN** the user opens the add-profile flow and searches or filters the common-provider catalog
- **THEN** the create dialog SHALL show only provider presets compatible with the selected Agent and matching the query or category
- **AND** SHALL retain a custom-provider entry

#### Scenario: Create a profile in a dialog
- **WHEN** the user selects a preset or custom provider in the add-profile flow
- **THEN** the create dialog SHALL populate an editable Agent-specific form below the preset selector
- **AND** SHALL keep cancel and save actions visible while the form scrolls
- **AND** SHALL neither save nor apply merely because a preset was selected

#### Scenario: Edit a profile in a dialog
- **WHEN** the user selects a profile edit action
- **THEN** the page SHALL open an accessible form-oriented edit dialog without requiring the source preset to be selected again
- **AND** SHALL never repopulate an existing credential value or apply the profile merely by saving it
- **AND** SHALL restore focus after the dialog closes and prevent duplicate submissions while saving

#### Scenario: Confirm a consequential profile action
- **WHEN** the user applies or deletes a profile, or performs a manual import requiring confirmation
- **THEN** the page SHALL use an application-owned confirmation dialog that identifies the profile and relevant effects
- **AND** SHALL not use a browser prompt or browser confirmation dialog

#### Scenario: Apply profile from the configuration page
- **WHEN** the user confirms a global profile application
- **THEN** the page SHALL show observable progress and the final restart or rollback guidance
- **AND** SHALL refresh profile status without changing the selected Session or runtime workflow

#### Scenario: Apply profile in Web mode
- **WHEN** a user applies a profile in the Web/mock runtime
- **THEN** the page SHALL show a deterministic simulated result without fabricating local files, credentials, or native runtime state

#### Scenario: Use the configuration page on a narrow viewport
- **WHEN** the page is rendered at a narrow supported viewport
- **THEN** the Agent switcher, configuration controls, status strip, toolbar, profile metadata, and card actions SHALL remain usable without horizontal page overflow
- **AND** create/edit dialogs SHALL keep their preset selector, form fields, and sticky primary actions keyboard-operable within the viewport
