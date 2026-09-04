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

#### Scenario: Render activity entries
- **WHEN** the activity rail renders
- **THEN** it SHALL show Sessions, Projects and Workspaces, Runs, Plan, and Quality as primary entries
- **AND** Settings and Help SHALL render in a separate anchored utility group
- **AND** Loops and Scheduled Tasks SHALL NOT render as separate primary entries; both are reachable through Runs' secondary navigation instead

#### Scenario: Identify icon-only entries
- **WHEN** an activity-bar entry is available to pointer, keyboard, or assistive-technology users
- **THEN** it SHALL provide an accessible name and tooltip synchronized across every registered application locale
- **AND** it SHALL expose stable hover, focus, selected, attention, and disabled styling without shifting adjacent entries

#### Scenario: Open settings from activity bar
- **WHEN** the user activates the Settings utility entry
- **THEN** the system SHALL open the existing settings center without requiring a runtime-specific backend call

#### Scenario: Open Loops from activity bar
- **WHEN** the user wants to open Loops
- **THEN** they SHALL do so through the Runs destination's Loops secondary route rather than a dedicated primary activity-bar entry
- **AND** the workspace SHALL preserve mounted Sessions destination state for later return

#### Scenario: Return to sessions from activity bar
- **WHEN** the user activates the Sessions primary entry while another destination is active
- **THEN** the workspace SHALL restore the Sessions destination without losing its selected session and mounted tab state

#### Scenario: Open scheduled tasks from activity bar
- **WHEN** the user wants to open Scheduled Tasks
- **THEN** they SHALL do so through the Runs destination's Schedules secondary route as a routed page rather than a dedicated primary activity-bar dialog entry
- **AND** it SHALL NOT show a coming-soon placeholder

#### Scenario: Preserve future help entry
- **WHEN** the activity rail renders its utility group
- **THEN** it SHALL keep the Help entry available without introducing a new Help destination in this requirement

#### Scenario: Open documentation from the Help utility entry
- **WHEN** the user activates the Help utility entry
- **THEN** the system SHALL open the settings center on the documentation page that renders the bundled product README
- **AND** it SHALL NOT open the About page as the Help destination

## ADDED Requirements

### Requirement: Workspace column separation
The workspace grid SHALL keep the session sidebar column visually and functionally separated from the conversation column at every supported viewport width, so that no sidebar content or affordance is overlapped by the conversation column.

#### Scenario: Sidebar trailing content stays visible
- **WHEN** the session sidebar is expanded at any supported width, including its minimum width
- **THEN** the trailing content of every session row, including its timestamp and any participant badge, SHALL remain fully visible within the sidebar column
- **AND** the conversation column SHALL NOT paint over sidebar content

#### Scenario: Resize affordance stays inside the sidebar column
- **WHEN** the user targets the session-sidebar resize control with a pointer or keyboard
- **THEN** the control SHALL be reachable within the sidebar column's own gutter
- **AND** activating it SHALL resize the sidebar within its supported bounds

#### Scenario: Sidebar overlay content is not clipped
- **WHEN** a menu or popover opened from inside the session sidebar extends toward the conversation column
- **THEN** it SHALL render above the conversation column instead of being covered by it

### Requirement: Embedded terminal frame integrity
An embedded terminal surface SHALL present one consistent frame, with nothing the terminal renderer paints escaping that frame, in every runtime the client ships on.

#### Scenario: Terminal corners are consistent
- **WHEN** an Agent CLI or Shell terminal renders inside the session workspace
- **THEN** all four corners of its frame SHALL share the same corner treatment
- **AND** the terminal renderer's own canvas SHALL NOT paint outside that frame, including where its row-sized canvas overhangs the host's content box

#### Scenario: Terminal frame is engine-independent
- **WHEN** the same terminal surface renders in the desktop runtime's web engine and in a browser runtime
- **THEN** its frame SHALL look the same in both
- **AND** it SHALL NOT depend on an engine clipping overflowing children to a border radius on its own

### Requirement: Structured create-session dialog
The create-session dialog SHALL present its choices as weighted, individually labeled sections ordered by decision dependency, and SHALL give each multi-Agent seat a distinguishable identity.

#### Scenario: Scan dialog sections
- **WHEN** the create-session dialog opens
- **THEN** it SHALL present participant selection, workspace selection, and session naming as separately labeled sections
- **AND** each section SHALL carry a localized heading distinguishable from field labels

#### Scenario: Identify a multi-Agent seat
- **WHEN** the dialog is in multi-Agent mode and shows the seat editor
- **THEN** each seat SHALL display its position, its selected Agent identity, and its selected expert role
- **AND** a seat whose role requires a different model family SHALL state that constraint next to that seat

#### Scenario: Preserve existing creation behavior
- **WHEN** the user completes the dialog in either single-Agent or multi-Agent mode
- **THEN** the submitted session input SHALL be unchanged by this presentation revision
- **AND** the dialog SHALL NOT introduce a control that assigns speaking order between seats

### Requirement: Session runtime failure recovery entry points
The workspace SHALL expose an explicit recovery action for a session whose runtime lifecycle is `failed`, reachable both from the session workspace and from the session list.

#### Scenario: Failure banner in the session workspace
- **WHEN** the displayed session's lifecycle state is `failed`
- **THEN** the session workspace SHALL show a banner identifying the failed runtime state and offering a recovery action
- **AND** the banner SHALL keep the reported failure reason visible rather than hiding it behind the action
- **AND** the banner SHALL be distinct from the crash-recovery acknowledgement notice

#### Scenario: Recover from the session list
- **WHEN** the user opens the context menu for a session that is not archived
- **THEN** the menu SHALL offer a recovery action for that session
- **AND** activating it SHALL recover that session without first switching the active session

#### Scenario: Report the recovery outcome
- **WHEN** a recovery action completes or fails
- **THEN** the workspace SHALL publish a session-scoped notification describing the outcome
- **AND** on success the session's displayed lifecycle state SHALL become idle without a manual refresh

#### Scenario: Recovery is not offered for archived sessions
- **WHEN** the context menu opens for an archived session
- **THEN** the recovery action SHALL NOT be offered
