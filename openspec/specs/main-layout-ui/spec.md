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
The sidebar SHALL support service-backed session navigation without the six bottom tool shortcuts and SHALL provide activity, folder, pinned, archived session organization, and dialog-based session creation.

#### Scenario: Preserve sidebar utility row
- **WHEN** the workspace sidebar is rendered
- **THEN** the sidebar SHALL keep bottom Settings and Help controls while omitting the six previous tool shortcuts and any visual style switching control

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

### Requirement: Flexible main content area
The main content panel SHALL render a chat-first workspace area that resizes with the available workspace area while keeping the bottom composer usable and connected to the active session message list.

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

#### Scenario: Message list renders for active session
- **WHEN** an active session is selected
- **THEN** the main content panel SHALL render the message list for that active session above the composer

#### Scenario: Composer sends to active session
- **WHEN** the user submits the bottom composer
- **THEN** the submitted chat message SHALL target the active session

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

### Requirement: Create-session dialog
The main layout UI SHALL provide a create-session dialog with Agent, project folder, project history, and optional Git worktree controls.

#### Scenario: Select Agent
- **WHEN** the create-session dialog opens
- **THEN** it SHALL let the user choose among Claude Code, Gemini CLI, Codex, and OpenCode using stable agent ids

#### Scenario: Show project history
- **WHEN** the create-session dialog opens
- **THEN** it SHALL show recently selected project folders from the frontend agent service

#### Scenario: Browse project folder
- **WHEN** the user chooses to browse for a project folder
- **THEN** the dialog SHALL request folder selection through the frontend agent service

#### Scenario: Show worktree controls for Git project
- **WHEN** the selected project folder is a Git repository
- **THEN** the dialog SHALL show an optional worktree checkbox and a worktree name field when the checkbox is enabled

#### Scenario: Disable worktree controls for non-Git project
- **WHEN** the selected project folder is not a Git repository
- **THEN** the dialog SHALL allow normal session creation and SHALL hide or disable worktree controls

#### Scenario: Submit concise failures
- **WHEN** project inspection, folder selection, or session creation fails
- **THEN** the dialog SHALL show a concise error message without rendering raw stdout or stderr

### Requirement: Polished workspace shell visuals
The workspace shell SHALL apply the shared visual design system consistently to the top bar, sidebar, main content panel, composer area, information panel, status bar, dialogs, and session cards.

#### Scenario: Workspace panel rhythm
- **WHEN** the workspace shell renders sidebar, main content, and information panel surfaces
- **THEN** panels SHALL use consistent border strength, panel backgrounds, radius, spacing, and shadow depth
- **AND** panel transitions and collapse controls SHALL remain visually aligned in both `futuristic` and `minimal` styles

#### Scenario: Session list visual hierarchy
- **WHEN** session cards, folder groups, activity groups, pinned areas, and archived areas render
- **THEN** they SHALL use consistent list-row density, icons, status markers, text hierarchy, hover states, and selected states
- **AND** long titles, folder paths, and agent labels SHALL not overlap adjacent controls

### Requirement: Workspace icon and toolbar polish
The workspace shell SHALL use consistent icons and compact toolbar controls for high-frequency workspace actions.

#### Scenario: Workspace action icons
- **WHEN** the top bar, sidebar utility row, session card context actions, create-session dialog, information panel tabs, or composer controls render actions
- **THEN** controls SHALL use consistent lucide or existing project icons where icons improve recognition
- **AND** icon-only controls SHALL have translated tooltips or accessible labels

#### Scenario: Compact grouped controls
- **WHEN** related workspace actions are displayed together
- **THEN** they SHALL use compact grouped-control styling with stable dimensions, consistent gaps, and clear active states
- **AND** hover or active styles SHALL not cause neighboring controls to shift

### Requirement: Workspace theme refinement
The workspace shell SHALL preserve functional layout behavior while improving visual quality in both registered styles.

#### Scenario: Futuristic workspace appearance
- **WHEN** `futuristic` style is active
- **THEN** the workspace SHALL present a dark, focused operational surface with subtle panel depth, readable transcript content, and clear primary/status accents

#### Scenario: Minimal workspace appearance
- **WHEN** `minimal` style is active
- **THEN** the workspace SHALL present a bright, crisp operational surface with low visual noise, clear separation between panels, and readable compact controls

### Requirement: Localized workspace shell text
The workspace shell SHALL render sidebar, status bar, information panel, session actions, and create-session dialog text through synchronized zh-CN and en translation resources.

#### Scenario: Create-session dialog localized
- **WHEN** the create-session dialog renders in Simplified Chinese or English
- **THEN** its title, description, project folder labels, browse action, Git/worktree helper text, worktree labels, session name labels, placeholders, create action, cancel action, and user-facing validation errors SHALL use the active locale

#### Scenario: Workspace panel labels localized
- **WHEN** the workspace shell renders sidebar, main content, information panel, status bar, or context menus in Simplified Chinese or English
- **THEN** user-visible labels, tab names, badges, context actions, confirmations, empty states, and helper text SHALL use the active locale

#### Scenario: Workspace date formatting localized
- **WHEN** workspace session cards or message-adjacent UI render user-visible dates
- **THEN** date formatting SHALL follow the active application language rather than always using a fixed locale

#### Scenario: Preserve workspace identifiers
- **WHEN** the workspace shell displays Agent ids, interaction mode ids, project paths, worktree names, branch names, or command-like values
- **THEN** those values MAY remain literal while surrounding labels and helper text use the active locale

### Requirement: IM session source identification
The workspace session navigation SHALL identify sessions created from IM bindings without exposing external identity values.

#### Scenario: Render IM-owned session
- **WHEN** a session has IM source metadata
- **THEN** its session card SHALL show a compact localized source indicator for Feishu, Telegram, DingTalk, WeCom, or personal WeChat alongside the existing Agent identity

#### Scenario: Protect external identifiers
- **WHEN** the session card or session details render an IM-owned session
- **THEN** they SHALL NOT display the raw external chat id, external user id, credentials, or authorization tokens

#### Scenario: Render in both styles
- **WHEN** an IM session indicator renders in `futuristic` or `minimal`
- **THEN** it SHALL use semantic tokens and stable dimensions without resizing, overlapping, or obscuring existing session actions

### Requirement: IM session actions remain consistent
IM-owned sessions SHALL use the existing session selection, rename, pin, archive, restore, and delete interactions.

#### Scenario: Select IM-owned session
- **WHEN** the user selects an IM-owned session card
- **THEN** the workspace SHALL display its persisted transcript through the existing Agent service

#### Scenario: Delete IM-owned session
- **WHEN** the user confirms deletion of an IM-owned session
- **THEN** the existing deletion interaction SHALL complete and the UI SHALL not require a platform-specific deletion flow
