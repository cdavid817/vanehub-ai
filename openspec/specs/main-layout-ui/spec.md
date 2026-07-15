# main-layout-ui Specification

## Purpose
Defines the workspace shell layout, sidebar session organization, main content sizing, collapsible information panel behavior, keep-alive panel tabs, and internal scrolling rules shared by the Tauri desktop frontend and browser Web runtime.

## Requirements
### Requirement: Three-panel workspace proportions
The workspace shell SHALL render sidebar, main content, and information panel as a unified three-panel layout with aligned heights.

#### Scenario: Render expanded panel layout
- **WHEN** the information panel is expanded
- **THEN** the workspace shell SHALL use panel proportions of 220px / 1fr / 300px

#### Scenario: Render collapsed panel layout
- **WHEN** the information panel is collapsed
- **THEN** the workspace shell SHALL use panel proportions of 220px / 1fr / 0px and the main content SHALL expand into the released space

#### Scenario: Align panel bottoms
- **WHEN** the workspace shell renders between the top bar and status bar
- **THEN** the sidebar, main content, and information panel SHALL use the same available height and align at the bottom edge

### Requirement: Sidebar session organization
The sidebar SHALL support service-backed session navigation without the six bottom tool shortcuts and SHALL provide activity, folder, pinned, and archived session organization.

#### Scenario: Preserve sidebar utility row
- **WHEN** the workspace sidebar is rendered
- **THEN** the sidebar SHALL keep bottom Settings, visual style switching, and Help controls while omitting the six previous tool shortcuts

#### Scenario: Show agent marker on session cards
- **WHEN** a session card is rendered
- **THEN** the card SHALL show an agent-type marker to the left of the title using a distinct icon and color for known agent types including Codex, Claude Code, OpenCode, and Gemini

#### Scenario: Create session from new action
- **WHEN** the user activates the sidebar new-session action
- **THEN** the sidebar SHALL create a session through the frontend agent service and make the created session available for selection

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

### Requirement: Flexible main content area
The main content panel SHALL render a chat-first workspace area that resizes with the available workspace area while keeping the bottom composer usable.

#### Scenario: Chat transcript flexes with panel height
- **WHEN** the workspace panel height changes
- **THEN** the chat transcript area SHALL flex to fill the remaining main content space without a fixed minimum height forcing overflow

#### Scenario: Chat transcript scrolls internally
- **WHEN** chat message content exceeds the available transcript height
- **THEN** the transcript SHALL scroll inside the main content panel without scrolling the whole workspace shell

#### Scenario: Composer remains fixed
- **WHEN** the main content panel becomes shorter
- **THEN** the bottom composer SHALL retain its usable size and SHALL remain within the main content panel bounds

#### Scenario: Main content expands after panel collapse
- **WHEN** the information panel is collapsed
- **THEN** the main content panel SHALL smoothly expand to occupy the space released by the information panel

### Requirement: Collapsible information panel
The information panel SHALL support smooth collapse and expand behavior while preserving mounted internal state.

#### Scenario: Collapse information panel
- **WHEN** the user clicks the information panel collapse control
- **THEN** the information panel SHALL collapse and the center panel SHALL expand using a 200ms CSS transition

#### Scenario: Show edge expand control
- **WHEN** the information panel is collapsed
- **THEN** the workspace SHALL show an expand control on the right edge that restores the panel when clicked

#### Scenario: Preserve panel component state
- **WHEN** the information panel is collapsed and later expanded
- **THEN** the panel SHALL preserve mounted component state including selected tab and form input values

#### Scenario: Scroll long panel content internally
- **WHEN** the active information panel content exceeds the panel height
- **THEN** the content area SHALL scroll inside the information panel without scrolling the whole workspace shell

### Requirement: Information panel tabs
The information panel SHALL provide keep-alive tabs for Agent Info, Files, and Changes.

#### Scenario: Render three tabs
- **WHEN** the information panel is rendered
- **THEN** the panel SHALL show exactly three tabs named Agent Info, Files, and Changes

#### Scenario: Switch tabs without unmounting content
- **WHEN** the user switches between information panel tabs
- **THEN** all tab contents SHALL remain mounted while only the selected tab content is visible

#### Scenario: Show agent progress summary
- **WHEN** the Agent Info tab is visible
- **THEN** the tab SHALL show an independent progress bar with overall completion percentage and completed, in-progress, and pending task counts
