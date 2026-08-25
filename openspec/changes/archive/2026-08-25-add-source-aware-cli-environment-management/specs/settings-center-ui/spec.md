## ADDED Requirements

### Requirement: CLI Management operational summary and filters

The CLI Management page SHALL present a compact operational summary and filters derived from normalized backend snapshots.

#### Scenario: Display summary

- **WHEN** CLI snapshots are available
- **THEN** the page SHALL show counts for ready, needs login, update available, conflicts, and broken
- **AND** it SHALL avoid oversized dashboard cards that reduce desktop information density

#### Scenario: Filter CLI tools

- **WHEN** the user searches or filters by status, source, or needs-action state
- **THEN** the visible tool list SHALL update without changing backend lifecycle policy
- **AND** filter and scroll state SHALL remain mounted across Settings navigation

### Requirement: Orthogonal CLI status presentation

The CLI Management page SHALL distinguish health, authentication, compatibility, update state, source, freshness, and manageability.

#### Scenario: Healthy detect-only installation

- **WHEN** a tool is healthy but its source is detect-only
- **THEN** the page SHALL show healthy executable status and detect-only management separately
- **AND** it SHALL not use a broken/error presentation merely because VaneHub cannot mutate it

#### Scenario: Tool needs authentication

- **WHEN** authentication is required
- **THEN** the page SHALL show needs-login/readiness state independently from installation and executable health

#### Scenario: Data is stale

- **WHEN** a snapshot is stale or refreshing
- **THEN** the page SHALL keep the last known values visible with freshness and last-checked information

### Requirement: Backend-authoritative CLI actions

The CLI Management page SHALL render only backend-derived allowed actions and SHALL not compare versions or infer lifecycle support in React.

#### Scenario: Primary action is rendered

- **WHEN** the backend returns one or more allowed actions
- **THEN** the page SHALL show one contextually appropriate primary action and place remaining actions in an accessible menu

#### Scenario: Current version is selected

- **WHEN** the selected target equals the active version
- **THEN** the page SHALL show current state
- **AND** no install, upgrade, or downgrade action SHALL be enabled

#### Scenario: No automatic action is allowed

- **WHEN** the backend returns no mutation action
- **THEN** the page SHALL show the backend reason and safe guidance rather than constructing a fallback action

### Requirement: CLI environment details drawer

The CLI Management page SHALL provide an accessible details drawer with Overview, Installations, Diagnostics, and Operations sections.

#### Scenario: View overview

- **WHEN** a user opens a tool's details
- **THEN** the drawer SHALL show overall and orthogonal states, active path/version/source, freshness, update source/channel, and last mutation outcome

#### Scenario: View installations

- **WHEN** the Installations section is selected
- **THEN** the drawer SHALL show bounded discovered paths, versions, sources, confidence, PATH priority, executable state, and active/shadowed identity

#### Scenario: View diagnostics

- **WHEN** the Diagnostics section is selected
- **THEN** the drawer SHALL show normalized version, Doctor, authentication, dependency, and compatibility results
- **AND** it SHALL provide a service-backed rerun action

#### Scenario: View operations

- **WHEN** the Operations section is selected
- **THEN** the drawer SHALL show related queued/running/terminal operations, phases, outcomes, timestamps, and bounded redacted logs

### Requirement: CLI action-plan review dialog

The UI SHALL require review of a persisted action plan before every CLI machine mutation.

#### Scenario: Review plan

- **WHEN** planning succeeds
- **THEN** the dialog SHALL show action, source, current and target versions, channel, structured command preview, network access, elevation, preconditions, warnings, expiry, and the absence of automatic source fallback

#### Scenario: Confirm plan

- **WHEN** the user confirms a non-stale plan
- **THEN** the page SHALL execute the plan id and revision through the service
- **AND** it SHALL not submit a command string or recompute the action

#### Scenario: Plan expires or becomes stale

- **WHEN** the backend rejects the plan as expired or stale
- **THEN** the dialog SHALL close or disable confirmation, explain that the environment changed, and offer preparation of a new plan

### Requirement: CLI bulk upgrade preview

The CLI Management page SHALL show a persisted bulk upgrade preview before starting any bulk mutation.

#### Scenario: Preview eligible items

- **WHEN** a bulk plan is prepared
- **THEN** the dialog SHALL list each eligible tool, source, current-to-target transition, elevation/network requirement, and warning

#### Scenario: Preview skipped items

- **WHEN** one or more tools are already current, detect-only, broken, unauthenticated, catalog-unavailable, or otherwise ineligible
- **THEN** the dialog SHALL list each skipped tool and localized reason

#### Scenario: Observe item outcomes

- **WHEN** bulk execution runs
- **THEN** the page SHALL show queued/running/terminal state and final outcome for every item
- **AND** one stale or failed item SHALL not erase other item results

### Requirement: CLI operation interaction

The CLI Management page SHALL display per-tool operation state, bounded logs, cancellation where supported, and partial-completion guidance.

#### Scenario: One tool operation runs

- **WHEN** one CLI operation is queued or running
- **THEN** only the affected tool SHALL show its active state
- **AND** unrelated tools SHALL remain inspectable

#### Scenario: Operation is cancellable

- **WHEN** the operation contract reports `cancellable`
- **THEN** the UI SHALL expose a cancel action through `OperationService`

#### Scenario: Mutation is applied but unverified

- **WHEN** the terminal result is applied-unverified
- **THEN** the UI SHALL state that the package command completed but verification failed
- **AND** it SHALL offer refresh or diagnostics without claiming rollback

#### Scenario: Command failed but machine changed

- **WHEN** the terminal result is changed-but-failed
- **THEN** the UI SHALL state that the machine appears to have changed despite the failed or cancelled command
- **AND** it SHALL display the observed snapshot and diagnostic action

### Requirement: Accessible and localized CLI Management

Every CLI Management control, state, dialog, drawer, tooltip, empty state, and error SHALL be accessible and localized in every registered application locale.

#### Scenario: Use keyboard and assistive technology

- **WHEN** a user navigates cards, menus, expanders, drawer tabs, dialogs, logs, or cancel actions by keyboard or assistive technology
- **THEN** focus order, focus restoration, labels, `aria-expanded`, `aria-controls`, dialog semantics, and non-color status cues SHALL be correct
- **AND** operation announcements SHALL not repeatedly announce streaming log lines

#### Scenario: Render registered locale

- **WHEN** the active application locale changes
- **THEN** all user-owned CLI Management labels and messages SHALL use matching locale resources
- **AND** dates SHALL use the active locale

#### Scenario: Render both visual styles

- **WHEN** `futuristic` or `minimal` style is active at desktop or narrow width
- **THEN** the page SHALL use shared semantic tokens, compact density, stable control dimensions, and no nested card-in-card layout
