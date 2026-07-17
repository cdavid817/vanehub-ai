## ADDED Requirements

### Requirement: Workspace activity bar
The workspace shell SHALL render a persistent icon-only activity bar at the far left of the workspace body in both the Tauri desktop frontend and browser Web runtime.

#### Scenario: Render activity entries
- **WHEN** the workspace activity bar renders
- **THEN** it SHALL show Session and Scheduled Tasks entries in a top group
- **AND** it SHALL show Settings and Help entries anchored in a bottom group
- **AND** the entries SHALL display icons without visible text labels

#### Scenario: Identify icon-only entries
- **WHEN** an activity-bar entry is available to pointer, keyboard, or assistive-technology users
- **THEN** it SHALL provide a synchronized zh-CN and en accessible name and tooltip
- **AND** it SHALL expose stable hover, focus, and active styling without shifting adjacent controls

#### Scenario: Open settings from activity bar
- **WHEN** the user activates the Settings activity entry
- **THEN** the system SHALL open the existing settings center without requiring a runtime-specific backend call

#### Scenario: Activate scheduled tasks placeholder
- **WHEN** the user activates the Scheduled Tasks activity entry
- **THEN** the workspace SHALL show a localized non-blocking coming-soon indication
- **AND** it SHALL NOT navigate, create a scheduled-task page, call a frontend service, or invoke native runtime behavior

#### Scenario: Preserve future help entry
- **WHEN** the activity bar renders its bottom group
- **THEN** it SHALL keep the Help entry available without introducing a new Help destination in this change

### Requirement: Collapsible session sidebar
The workspace shell SHALL let the Session activity entry collapse and expand the session sidebar while preserving the sidebar component's mounted state.

#### Scenario: Render default session sidebar state
- **WHEN** the workspace is opened or reloaded
- **THEN** the session sidebar SHALL be expanded by default
- **AND** the Session activity entry SHALL expose the sidebar's expanded state to assistive technology

#### Scenario: Collapse session sidebar
- **WHEN** the user activates the Session activity entry while the session sidebar is expanded
- **THEN** the sidebar SHALL collapse using a 200ms layout transition
- **AND** the main content SHALL expand into the released 220px width
- **AND** hidden sidebar controls SHALL NOT remain reachable by pointer, keyboard, or assistive technology

#### Scenario: Expand session sidebar
- **WHEN** the user activates the Session activity entry while the session sidebar is collapsed
- **THEN** the sidebar SHALL expand to 220px using a 200ms layout transition

#### Scenario: Preserve session sidebar state
- **WHEN** the user collapses and later expands the session sidebar
- **THEN** the sidebar SHALL preserve mounted state including the selected activity, group, or archived view and expanded folder groups

#### Scenario: Collapse panels independently
- **WHEN** the session sidebar or information panel is collapsed or expanded
- **THEN** each panel state SHALL change independently without resetting or forcing the other panel state

#### Scenario: Keep activity bar available at responsive widths
- **WHEN** the workspace width is at or below 900px or 640px
- **THEN** the activity bar SHALL remain visible and the Session entry SHALL remain operable
- **AND** the existing responsive information-panel hiding and bounded single-column session-sidebar behavior SHALL remain usable

## MODIFIED Requirements

### Requirement: Three-panel workspace proportions
The workspace shell SHALL render a fixed-width activity bar beside a sidebar, main content, and information panel grid whose visible panels use aligned heights.

#### Scenario: Render expanded panel layout
- **WHEN** the session sidebar and information panel are expanded
- **THEN** the workspace grid SHALL use panel proportions of 220px / 1fr / 300px beside the activity bar

#### Scenario: Render collapsed information panel layout
- **WHEN** the session sidebar is expanded and the information panel is collapsed
- **THEN** the workspace grid SHALL use panel proportions of 220px / 1fr / 0px and the main content SHALL expand into the released space

#### Scenario: Render collapsed session sidebar layout
- **WHEN** the session sidebar is collapsed and the information panel is expanded
- **THEN** the workspace grid SHALL use panel proportions of 0px / 1fr / 300px and the main content SHALL expand into the released space

#### Scenario: Render both panels collapsed
- **WHEN** the session sidebar and information panel are collapsed
- **THEN** the workspace grid SHALL use panel proportions of 0px / 1fr / 0px and the main content SHALL occupy all released grid space

#### Scenario: Align panel bottoms
- **WHEN** the workspace shell renders between the top bar and status bar
- **THEN** the activity bar and all visible workspace panels SHALL use the same available height and align at the bottom edge

### Requirement: Sidebar session organization
The sidebar SHALL support service-backed session navigation without utility or tool shortcuts and SHALL provide activity, folder, pinned, archived session organization, and dialog-based session creation.

#### Scenario: Omit sidebar utility row
- **WHEN** the workspace sidebar is rendered
- **THEN** the sidebar SHALL omit Settings, Help, the six previous tool shortcuts, and any visual style switching control because global utility actions are owned by the activity bar

#### Scenario: Show agent marker on session cards
- **WHEN** a session card is rendered
- **THEN** the card SHALL show an agent-type marker to the left of the title using a distinct icon and color for known agent types including Codex, Claude Code, OpenCode, and Gemini

#### Scenario: Open create-session dialog from new action
- **WHEN** the user activates the sidebar new-session action
- **THEN** the sidebar SHALL open a create-session dialog rather than immediately creating a session

#### Scenario: Create session from dialog
- **WHEN** the user submits a valid create-session dialog
- **THEN** the UI SHALL create a session through the frontend agent service and make the created session available for selection

#### Scenario: Select session card
- **WHEN** the user selects a session card
- **THEN** the sidebar SHALL switch the active session through the frontend agent service and visually mark that card as selected

#### Scenario: Switch to activity view
- **WHEN** the user selects the activity view mode
- **THEN** the sidebar SHALL group sessions into needs-input, pending-verification, in-progress, and inactive categories with a visible count for each category

#### Scenario: Sort activity groups by priority
- **WHEN** the sidebar renders the activity view mode
- **THEN** the categories SHALL appear in priority order: needs-input, pending-verification, in-progress, inactive

#### Scenario: Show pinned sessions
- **WHEN** one or more sessions are pinned
- **THEN** the sidebar SHALL render pinned sessions in a dedicated pinned area before the normal activity or folder groups

#### Scenario: Switch to folder group view
- **WHEN** the user selects the group view mode
- **THEN** the sidebar SHALL group sessions by their owning folder

#### Scenario: Toggle folder expansion
- **WHEN** the user toggles a folder group in group view
- **THEN** the sidebar SHALL expand or collapse that folder's session cards without changing the selected session

#### Scenario: Open archived view
- **WHEN** the user opens the archived session view
- **THEN** the sidebar SHALL show archived sessions from the frontend agent service and indicate the archived session count

#### Scenario: Use context actions
- **WHEN** the user opens a session card context menu
- **THEN** the sidebar SHALL provide actions to rename, pin or unpin, archive or restore, and delete the session according to the session's current state

#### Scenario: Prevent browser context menu
- **WHEN** the user opens the custom session context menu in browser or desktop WebView mode
- **THEN** the sidebar SHALL prevent the browser default context menu from appearing over the custom menu

#### Scenario: Confirm destructive session deletion
- **WHEN** the user chooses to delete a session
- **THEN** the sidebar SHALL ask for confirmation before calling the delete operation

#### Scenario: Scroll long session lists internally
- **WHEN** the session list content exceeds the sidebar height
- **THEN** the session list SHALL scroll inside the sidebar without scrolling the whole workspace shell

### Requirement: Workspace icon and toolbar polish
The workspace shell SHALL use consistent icons and compact toolbar controls for high-frequency workspace actions.

#### Scenario: Workspace action icons
- **WHEN** the top bar, activity bar, session card context actions, create-session dialog, information panel tabs, or composer controls render actions
- **THEN** controls SHALL use consistent lucide or existing project icons where icons improve recognition
- **AND** icon-only controls SHALL have translated tooltips or accessible labels

#### Scenario: Compact grouped controls
- **WHEN** related workspace actions are displayed together
- **THEN** they SHALL use compact grouped-control styling with stable dimensions, consistent gaps, and clear active states
- **AND** hover or active styles SHALL not cause neighboring controls to shift
