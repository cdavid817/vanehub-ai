## ADDED Requirements

### Requirement: Declarative session workspace tab capabilities

The session workspace SHALL define each tab's seat scope, live-update support, and mounted-retention policy in one declarative capability registry rather than duplicating tab-id behavior across components.

#### Scenario: Render a seat-optional tab

- **WHEN** Terminal History or Logs is active for a multi-Agent session
- **THEN** the workspace SHALL expose `all seats` and concrete active-seat choices according to the tab capability
- **AND** the selected seat SHALL be included in the tab's service query key

#### Scenario: Render a seat-required tab

- **WHEN** Shell is active for a multi-Agent session
- **THEN** the workspace SHALL require one concrete active seat before creating or attaching a Shell

#### Scenario: Render a session-scoped tab

- **WHEN** Changes, Documents, Files, Traces, or Report is active in its default mode
- **THEN** the workspace SHALL not show a global seat control that appears to filter that tab

#### Scenario: Add a future workspace tab

- **WHEN** a future tab is registered
- **THEN** its scope and lifecycle behavior SHALL be declared in the same registry
- **AND** React SHALL not infer those semantics from its translated label or display order

### Requirement: Shared workspace evidence navigation

Workspace panels SHALL navigate to correlated evidence through one shared target containing the destination tab and serializable evidence scope.

#### Scenario: Open a command from Traces

- **WHEN** a selected span exposes a command target
- **THEN** the workspace SHALL activate Terminal History and focus the command record
- **AND** the selected session, run, trace, span, operation, command, and seat fields available in the target SHALL remain visible as active scope

#### Scenario: Clear a cross-panel filter

- **WHEN** the user clears an active run, span, operation, command, path, or timestamp filter in a destination panel
- **THEN** the shared scope SHALL remove that field without resetting unrelated panel state or the selected session

#### Scenario: Change sessions after cross-panel navigation

- **WHEN** a different session is selected
- **THEN** evidence ids owned by the previous session SHALL be cleared before destination queries run

### Requirement: Evidence-aware workspace tab badges

The session workspace tab row SHALL display bounded service-backed badges or status markers for Changes, Terminal History, Shell, Logs, Traces, and Report without mounting every panel's full query.

#### Scenario: Show actionable tab badges

- **WHEN** the selected session has unviewed review files, running or failed execution records, live Shells, new error logs, running or failed runs, failed verification, or partial report coverage
- **THEN** the owning tab SHALL show a compact localized count or status marker from the workspace evidence summary

#### Scenario: Badge value is zero

- **WHEN** a numeric badge count is zero
- **THEN** the tab SHALL omit the numeric badge unless a non-count warning state remains actionable

#### Scenario: Badge source is incomplete

- **WHEN** a summary source is indexing, partial, or unavailable
- **THEN** the badge or tooltip SHALL expose that coverage state
- **AND** it SHALL not display an unknown count as a definitive zero

#### Scenario: Read badge with assistive technology

- **WHEN** an icon, color, or compact badge communicates a workspace state
- **THEN** it SHALL provide a localized accessible name describing the tab and state
- **AND** state SHALL not depend on color alone

### Requirement: Mounted panel state with suspended hidden work

Visited workspace tabs and information-panel panes SHALL preserve mounted local state as currently specified, while live subscriptions, polling, and background refresh SHALL be suspended when a panel is hidden unless an in-flight mutation must finish.

#### Scenario: Hide Logs or Traces

- **WHEN** a mounted Logs or Traces panel becomes hidden
- **THEN** it SHALL unsubscribe or suspend its live stream and periodic refresh
- **AND** its loaded rows, filters, selection, and scroll state SHALL remain available for later return

#### Scenario: Hide Shell

- **WHEN** a mounted Shell panel becomes hidden
- **THEN** its xterm view SHALL detach from the native Shell stream
- **AND** the native retained Shell SHALL remain live according to its own lifecycle policy

#### Scenario: Hide an information-panel pane

- **WHEN** the user switches from one mounted information-panel pane to another
- **THEN** the inactive pane SHALL preserve local form/selection state
- **AND** service queries or subscriptions unnecessary while hidden SHALL be disabled

#### Scenario: Mutation is still running

- **WHEN** a hidden panel owns an already-started service mutation
- **THEN** the mutation MAY continue through its backend operation contract
- **AND** hiding the panel SHALL not discard its terminal outcome or error

### Requirement: Execution-record Terminal History presentation

The Terminal History workspace tab SHALL present evidence-backed Commands, Tools, Delegations, Verification, and explicitly labelled Legacy Activity through bounded filters, a virtualized record list, and a safe detail surface.

#### Scenario: Render mixed execution records

- **WHEN** a session contains native commands, proxied tools, delegated work, verification outcomes, and legacy message activity
- **THEN** the tab SHALL let the user filter those record kinds
- **AND** every row SHALL display its observed status, fidelity, timing availability, seat/run correlation, and coverage without fabricating missing fields

#### Scenario: Open command detail

- **WHEN** a command row is selected
- **THEN** the detail surface SHALL show runtime kind, bounded redacted display availability, working-directory display, duration, exit/signal data, output availability/truncation, correlation, and evidence actions when available
- **AND** it SHALL distinguish merged PTY output from separate stdout/stderr

#### Scenario: Append another record page fails

- **WHEN** records are visible and a continuation request fails
- **THEN** the visible records SHALL remain mounted
- **AND** an inline Retry action SHALL appear at the continuation boundary

#### Scenario: Render maximum accepted records

- **WHEN** loaded execution records reach the configured UI bound
- **THEN** the tab SHALL virtualize rows so mounted record articles remain bounded by the viewport

### Requirement: Evidence-aware Basic Info summary

The Basic Info pane SHALL include a compact service-backed summary of current runtime, workspace provider/Git state, retained Shells, changes/review progress, verification, diagnostics, and usage coverage for the selected session.

#### Scenario: Display a running session summary

- **WHEN** a selected session has current evidence
- **THEN** Basic Info SHALL display bounded status rows for available runtime duration/state, workspace provider and dirty state, live Shell count, changed/unviewed counts, verification totals, diagnostic error/retry counts, and usage quality/coverage

#### Scenario: Navigate from a summary row

- **WHEN** the user activates Changes, Shells, Diagnostics, Verification, Usage, or another actionable summary row
- **THEN** the workspace SHALL navigate to the owning tab and relevant evidence scope

#### Scenario: Summary section is unavailable

- **WHEN** one source cannot provide a current summary
- **THEN** Basic Info SHALL show an unavailable/partial state for that row while preserving the other rows
- **AND** it SHALL not replace unknown values with definitive zeroes

#### Scenario: Preserve existing information panes

- **WHEN** the evidence-aware summary is added
- **THEN** existing Basic Info, Token Usage, Skill, optional Member Information, IM, and Code Index behavior SHALL remain available according to their existing eligibility rules

### Requirement: Evidence workspace responsive and accessible presentation

The upgraded workspace panels SHALL remain usable in desktop and narrow layouts, in `futuristic` and `minimal` styles, using semantic tokens, compact operational density, synchronized locale resources, and keyboard-accessible controls.

#### Scenario: Render at desktop width

- **WHEN** the evidence workspace renders at desktop width
- **THEN** record lists, file/document navigation, log filters, trace waterfall, report sections, review progress, and detail surfaces SHALL use available space without forcing whole-workspace scrolling

#### Scenario: Render at narrow width

- **WHEN** the workspace renders at a narrow supported width
- **THEN** secondary rails and detail panes SHALL become drawers, switchable regions, or vertically bounded areas
- **AND** primary actions, active filters, and current status SHALL remain reachable

#### Scenario: Use localized long labels

- **WHEN** a registered locale produces labels longer than their controls
- **THEN** controls SHALL truncate or wrap within their declared region while preserving full accessible names/tooltips
- **AND** they SHALL not resize the workspace grid or overlap adjacent actions

#### Scenario: Change loading or status state

- **WHEN** a control changes among idle, loading, live, warning, failure, disabled, or selected states
- **THEN** its dimensions SHALL remain stable and adjacent controls SHALL not shift
