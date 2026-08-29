# main-layout-ui Specification Delta

## ADDED Requirements

### Requirement: Session runtime panel
The Sessions workspace SHALL provide a resizable bottom Runtime Panel for Terminal History, Shell, Logs, and Traces with per-session state and bounded background behavior.

#### Scenario: Open runtime evidence
- **WHEN** the user opens a runtime surface from a badge, command, slash request, evidence link, or panel tab
- **THEN** the Runtime Panel SHALL open to that surface and preserve the active Session and primary surface

#### Scenario: Resize or maximize
- **WHEN** the user resizes or maximizes the Runtime Panel
- **THEN** the panel SHALL clamp to a usable range and preserve an accessible restore action
- **AND** the main surface SHALL not be reduced below its documented minimum height

#### Scenario: Switch sessions
- **WHEN** the active Session changes
- **THEN** the Runtime Panel SHALL reconcile to that Session's permitted surfaces and seat scope
- **AND** it SHALL not show evidence from the previous Session under the new title

#### Scenario: Close the panel
- **WHEN** the Runtime Panel closes
- **THEN** service-owned terminals or Runs SHALL follow their existing ownership semantics
- **AND** page-owned polling and rendering SHALL follow the declared hidden policy

### Requirement: Session route and return-context compatibility
Session and evidence navigation SHALL use validated route state and preserve a safe return context when entering a Session from Runs, Loops, Plan, Quality, or Projects.

#### Scenario: Open session evidence from another destination
- **WHEN** an owning surface navigates to a Session, primary surface, runtime surface, or evidence selection
- **THEN** the Sessions route SHALL validate ownership and display the requested target
- **AND** a safe internal return context SHALL be available

#### Scenario: Requested evidence is stale
- **WHEN** the route references deleted, unavailable, restricted, or cross-session evidence
- **THEN** the workspace SHALL show an explicit unavailable or restricted state
- **AND** it SHALL not silently display unrelated default evidence

#### Scenario: Return to owning surface
- **WHEN** the user activates Back to source
- **THEN** the original destination, selected entity, supported filters, and scroll anchor SHALL be restored when still valid

### Requirement: Contextual session status hierarchy
The session header, turn status, message state, and action controls SHALL derive from one presentation model that distinguishes session lifecycle, current execution, participant turn, recovery, and evidence status.

#### Scenario: Render an active turn
- **WHEN** a participant or CLI execution is active
- **THEN** the header SHALL show one bounded authoritative running summary and the state-appropriate primary action
- **AND** message-local state SHALL remain attached to the affected message without duplicating the full header summary

#### Scenario: Render conflicting source updates
- **WHEN** message, stream, and session projections temporarily disagree during reconciliation
- **THEN** the presentation model SHALL prefer canonical terminal and recovery states according to documented precedence
- **AND** the UI SHALL indicate refreshing rather than showing contradictory primary statuses

#### Scenario: Render no active execution
- **WHEN** the Session is idle
- **THEN** the header SHALL not reserve space for an empty running banner or duplicate neutral badges

## MODIFIED Requirements

### Requirement: Workspace activity bar
The workspace shell SHALL render a persistent primary activity rail for Sessions, Projects and Workspaces, Runs, Plan, and Quality, with Settings and Help anchored in a utility group, in both Tauri and Web runtimes.

#### Scenario: Render business destinations
- **WHEN** the activity rail renders
- **THEN** it SHALL show icon entries for Sessions, Projects and Workspaces, Runs, Plan, and Quality in the primary group
- **AND** Settings and Help SHALL remain in the utility group

#### Scenario: Identify an entry
- **WHEN** an activity entry is available to pointer, keyboard, or assistive-technology users
- **THEN** it SHALL provide a localized accessible name and tooltip
- **AND** hover, focus, selected, attention, and disabled states SHALL not shift adjacent entries

#### Scenario: Open grouped capabilities
- **WHEN** the user opens Runs, Plan, or Quality
- **THEN** the destination SHALL present its own localized secondary navigation
- **AND** the activity rail SHALL NOT duplicate Loops, Schedules, Board, Goals, Mission Control, or Evaluation as additional primary entries

#### Scenario: Use a narrow height
- **WHEN** the available vertical space is limited
- **THEN** all five primary entries and the utility group SHALL remain reachable without an undiscoverable clipped item

### Requirement: Three-panel workspace proportions
The Sessions workspace SHALL use a bounded resizable context-navigation pane, a minimum-width main work surface, and an optional bounded Inspector whose presentation changes between inline and sheet modes according to available container width.

#### Scenario: Render wide session workspace
- **WHEN** the workbench has sufficient wide-layout space and both auxiliary panes are preferred open
- **THEN** the session navigation SHALL render between 256px and 400px and the Inspector between 320px and 480px
- **AND** the main surface SHALL receive the remaining width without overlap

#### Scenario: Protect main content width
- **WHEN** the three inline panes would make the main surface narrower than the documented minimum
- **THEN** the Inspector SHALL become a sheet or close before the session navigation compresses below its minimum
- **AND** the session navigation SHALL become a sheet if the main surface still cannot remain usable

#### Scenario: Resize a pane
- **WHEN** the user adjusts an inline pane
- **THEN** the resize affordance SHALL stay inside a reserved gutter and clamp the value
- **AND** the preference SHALL persist without storing sensitive content

#### Scenario: Use focus mode
- **WHEN** the user enters conversation focus mode
- **THEN** auxiliary panes and optional navigation SHALL hide without losing their prior preferred states
- **AND** the user SHALL retain an always-reachable exit control

### Requirement: Sidebar session organization
The Sessions context-navigation pane SHALL provide an attention-first, virtualized, service-backed session list with compact view, filter, archive, category, project, pin, search, batch, and context-action capabilities.

#### Scenario: Render the default view
- **WHEN** sessions are available and no explicit saved view is selected
- **THEN** the pane SHALL group or rank needs-attention, running, pinned, recent, and remaining sessions before lower-priority history

#### Scenario: Use alternate organization
- **WHEN** the user chooses category, project, archived, or flat organization
- **THEN** the pane SHALL preserve that view while search or filters change
- **AND** returning from a selected session SHALL retain group expansion and scroll anchor

#### Scenario: Keep controls compact
- **WHEN** search, view selection, filters, archive, and batch management are available
- **THEN** search and the new-session action SHALL remain obvious
- **AND** secondary controls SHALL use a bounded toolbar or overflow rather than permanent full-width rows

#### Scenario: Render a large history
- **WHEN** the visible source contains at least one thousand sessions
- **THEN** the pane SHALL virtualize or page session rows while preserving stable selection, keyboard navigation, context menus, and grouped counts

#### Scenario: Render a session row
- **WHEN** a session appears in the default list
- **THEN** the row SHALL prioritize Agent or role identity, title, actionable state, relative time, and at most one bounded secondary line
- **AND** optional metadata badges SHALL follow a documented display budget and SHALL NOT cause horizontal scrolling

#### Scenario: Enter batch mode
- **WHEN** the user starts batch management
- **THEN** a dedicated batch action region SHALL show selected count and permitted actions
- **AND** normal session activation and drag behavior SHALL be suspended until batch mode exits

### Requirement: Optimized information panel tabs
The former fixed session information-tab panel SHALL become a contextual Inspector with Session Overview, Follow Selection, and Pinned modes. Session overview content SHALL use bounded sections rather than a row of equal-width Basic, Member, Usage, Skill, IM, or Code Index tabs.

#### Scenario: Show session overview
- **WHEN** an active session has no more specific selection or the user chooses Overview
- **THEN** the Inspector SHALL show available Participant, Runtime, Usage, Skill, Workspace, IM, and Code Index summaries in bounded sections
- **AND** unavailable sections SHALL identify why they are unavailable

#### Scenario: Follow a selected object
- **WHEN** the user selects a message, tool call, file, change, Run reference, or other supported evidence object
- **THEN** the Inspector SHALL load that object's bounded detail through its owning provider
- **AND** it SHALL retain a route to the authoritative full surface

#### Scenario: Pin inspector content
- **WHEN** the user pins the current Inspector selection
- **THEN** subsequent main-surface selections SHALL not replace it
- **AND** the pinned header SHALL identify the pinned object and provide unpin and overview actions

#### Scenario: Use inspector as a sheet
- **WHEN** available width does not support an inline Inspector
- **THEN** the same overview and selected-object content SHALL open in an accessible sheet
- **AND** closing the sheet SHALL return focus to the source object

#### Scenario: Protect panel performance
- **WHEN** the Inspector is closed or a section has not been expanded
- **THEN** it SHALL not start unneeded high-frequency queries or mount every detail implementation
- **AND** service-owned background work SHALL remain unaffected

### Requirement: Create-session dialog
The main layout UI SHALL provide a four-step create-session wizard for runtime mode, participants and capabilities, workspace, and final review, using the shared application dialog or full-height sheet primitive according to available width.

#### Scenario: Choose runtime mode
- **WHEN** the wizard opens
- **THEN** the first step SHALL let the user choose supported Single or Multi Agent, CLI or API, and Local or Remote modes
- **AND** unsupported combinations SHALL be disabled with an explanation instead of failing only at submit

#### Scenario: Configure participants
- **WHEN** the participant step renders
- **THEN** it SHALL show each selected Agent, role, model-family compatibility, and relevant personalization or Skill summary
- **AND** advanced options SHALL remain progressively disclosed

#### Scenario: Choose workspace
- **WHEN** the workspace step renders
- **THEN** it SHALL let the user select a recent or discovered project or remote workspace and configure branch or worktree options when supported
- **AND** trust, filesystem, network, and destructive consequences SHALL be visible before proceeding

#### Scenario: Review creation
- **WHEN** required inputs are valid
- **THEN** the review step SHALL summarize runtime, participants, workspace, profile overrides, risks, and resulting resources
- **AND** the user SHALL be able to return to each previous step without losing valid input

#### Scenario: Show validation errors
- **WHEN** a field, discovery operation, trust check, or creation request fails
- **THEN** the wizard SHALL show an error adjacent to the affected field or step and a summary at the review surface
- **AND** the error SHALL remain reachable when content scrolls and SHALL be announced

#### Scenario: Submit once
- **WHEN** session creation is pending
- **THEN** duplicate submission and destructive dismissal SHALL be prevented
- **AND** scrolling, copying the review, and accessing non-conflicting help SHALL remain available

### Requirement: Declarative session workspace tab capabilities
The session workspace SHALL declare four primary surfaces and four runtime-panel surfaces in one capability registry, including scope, retention, live-update, badge, route-compatibility, and renderer policies.

#### Scenario: Render primary surfaces
- **WHEN** an active session workspace renders
- **THEN** it SHALL provide stable primary slots for Work, Changes, Files, and Report
- **AND** the Work renderer SHALL show structured conversation for API or shared sessions and the existing Agent Terminal for supported single-Agent CLI sessions

#### Scenario: Render runtime surfaces
- **WHEN** runtime evidence or tools are available
- **THEN** Terminal History, Shell, Logs, and Traces SHALL be exposed through the Runtime Panel rather than the primary tab strip

#### Scenario: Merge document and file discovery
- **WHEN** the user opens Files
- **THEN** the surface SHALL offer document and explorer views without requiring two permanent primary tabs
- **AND** existing document and file service behavior SHALL remain reachable

#### Scenario: Resolve a legacy tab request
- **WHEN** a slash command, deep link, stored preference, or owning surface requests chat, documents, files, terminal, shell, logs, traces, changes, or report
- **THEN** the compatibility adapter SHALL map the request to the corresponding primary surface, runtime surface, and subview
- **AND** new internal code SHALL use the new surface ids

#### Scenario: Apply seat scope
- **WHEN** a registered surface is session-scoped, seat-optional, or seat-required
- **THEN** the workspace SHALL render only the seat selector appropriate to that declaration
- **AND** query keys and evidence scope SHALL include the validated seat identity when applicable

#### Scenario: Suspend a hidden surface
- **WHEN** a surface is not visible
- **THEN** its registry retention and live-update policies SHALL control mounting and subscriptions
- **AND** CSS visibility alone SHALL NOT be treated as proof that expensive work is suspended
