# main-layout-ui Specification

## Purpose
Defines the workspace shell layout, sidebar session organization, main content sizing, collapsible information panel behavior, keep-alive panel tabs, and internal scrolling rules shared by the Tauri desktop frontend and browser Web runtime.
## Requirements
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

### Requirement: Main content Agent workspace
The main content panel SHALL render a Workspace-first area for active single-Agent CLI sessions while keeping the panel responsive within the workspace shell.

#### Scenario: Workspace tab is user-facing
- **WHEN** the session tab navigation renders for the former Agent Terminal surface
- **THEN** the tab SHALL be named Workspace / 工作区
- **AND** the surface SHALL continue to host the selected Agent CLI terminal interaction

#### Scenario: Workspace terminal composer
- **WHEN** a Workspace terminal session is attached
- **THEN** the workspace SHALL provide a bottom multiline composer below the terminal viewport
- **AND** pressing Enter in the composer SHALL send the entered text followed by Enter to the current Agent CLI terminal
- **AND** pressing Shift+Enter SHALL insert a new line without submitting
- **AND** the composer SHALL be disabled when no terminal process is attached

#### Scenario: Agent Terminal flexes with panel height
- **WHEN** the workspace panel height changes
- **THEN** the Agent Terminal area SHALL flex to fill the available main content space without a fixed minimum height forcing overflow

#### Scenario: Agent Terminal scrolls internally
- **WHEN** terminal content exceeds the available terminal viewport
- **THEN** the terminal SHALL scroll or buffer inside the main content panel without scrolling the whole workspace shell

#### Scenario: Main content expands after panel collapse
- **WHEN** the information panel is collapsed
- **THEN** the main content panel SHALL smoothly expand to occupy the space released by the information panel

#### Scenario: Agent Terminal renders for active session
- **WHEN** an active single-Agent CLI session is selected
- **THEN** the main content panel SHALL render the Agent Terminal for that active session instead of the previous chat message list and composer

#### Scenario: Session-page chat selectors removed
- **WHEN** the Agent Terminal main content renders
- **THEN** the page SHALL NOT render model, provider, permission, reasoning, thinking, streaming, or prompt-composer controls for that terminal

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
The information panel SHALL provide keep-alive tabs for Basic Info, Files, Changes, and Logs.

#### Scenario: Information panel tab set
- **WHEN** the information panel renders for an active session
- **THEN** the panel SHALL show tabs named Basic Info, Files, Changes, and Logs
- **AND** the previous Agent Info tab content SHALL remain available under Basic Info

#### Scenario: Switch tabs without unmounting content
- **WHEN** the user switches between information panel tabs
- **THEN** all tab contents SHALL remain mounted while only the selected tab content is visible

#### Scenario: Show agent progress summary
- **WHEN** the Basic Info tab is visible
- **THEN** the tab SHALL show an independent progress bar with overall completion percentage and completed, in-progress, and pending task counts

#### Scenario: Compact terminal logs are visible
- **WHEN** the user opens the Logs tab in the information panel
- **THEN** the panel SHALL show recent session log entries for Agent terminal diagnostics
- **AND** startup and startup-failure records SHALL be visible without opening the detailed Logs workspace tab

### Requirement: Create-session dialog
The main layout UI SHALL provide a create-session dialog with Agent mode selection, Agent choice for Single Agent sessions, project folder, project history, and optional Git worktree controls.

#### Scenario: Select session mode
- **WHEN** the create-session dialog opens
- **THEN** it SHALL present Single Agent and Multi Agent mode choices
- **AND** Single Agent SHALL be the enabled first-version mode

#### Scenario: Multi Agent is disabled
- **WHEN** the user views the Multi Agent mode choice
- **THEN** it SHALL be marked as coming soon or disabled
- **AND** the user SHALL NOT be able to submit a Multi Agent session

#### Scenario: Select Agent
- **WHEN** Single Agent mode is active
- **THEN** the dialog SHALL let the user choose among Claude Code, Gemini CLI, Codex, and OpenCode using stable agent ids

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
- **THEN** the dialog SHALL allow normal Single Agent session creation and SHALL hide or disable worktree controls

#### Scenario: Submit concise failures
- **WHEN** project inspection, folder selection, or session creation fails
- **THEN** the dialog SHALL show a concise error message without rendering raw stdout or stderr

### Requirement: Agent Terminal and Shell tab separation
The workspace shell SHALL keep the Agent Terminal experience separate from the ordinary project Shell tab.

#### Scenario: Keep ordinary Shell tab
- **WHEN** an active session is selected
- **THEN** the workspace SHALL keep the existing ordinary Shell tab available for project shell commands
- **AND** that Shell tab SHALL NOT inject Agent CLI parameters or automatically launch the selected Agent CLI

#### Scenario: Agent Terminal owns Agent CLI interaction
- **WHEN** the user interacts with the selected Agent CLI
- **THEN** that interaction SHALL occur through the Agent Terminal surface
- **AND** it SHALL use the selected session's stable agent id

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
- **WHEN** the top bar, activity bar, session card context actions, create-session dialog, information panel tabs, or composer controls render actions
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

### Requirement: Sidebar session search
The workspace sidebar SHALL provide a localized historical session search entry point.

#### Scenario: Search sessions from sidebar
- **WHEN** the user enters a non-empty search query in the session sidebar
- **THEN** the sidebar SHALL show bounded matching sessions from the frontend service with title, agent marker, project metadata, category, archived state, and updated timestamp

#### Scenario: Clear search
- **WHEN** the user clears the search query
- **THEN** the sidebar SHALL return to the previously selected session organization view without discarding selected session state

### Requirement: Sidebar category view
The workspace sidebar SHALL support a category organization view backed by user-defined session categories.

#### Scenario: Render category groups
- **WHEN** the user selects category view
- **THEN** the sidebar SHALL group sessions by assigned category and SHALL include a localized uncategorized group for sessions without a category

#### Scenario: Toggle category expansion
- **WHEN** the user toggles a category group
- **THEN** the sidebar SHALL expand or collapse that category's session cards without changing the active session

### Requirement: Session category context actions
The session card context menu SHALL let users move sessions between categories and create categories.

#### Scenario: Move to existing category
- **WHEN** the user chooses a category from a session card context menu
- **THEN** the sidebar SHALL call the frontend service to assign the selected session to that category

#### Scenario: Create category from session menu
- **WHEN** the user chooses to create a category from a session card context menu and submits a valid name
- **THEN** the sidebar SHALL create the category through the frontend service and move the session to it

### Requirement: Drag session to category
The sidebar SHALL support dragging a session card onto a category group to assign that category.

#### Scenario: Drop session on category
- **WHEN** the user drops a session card on a category group
- **THEN** the sidebar SHALL assign that session to the target category through the frontend service

#### Scenario: Accessible non-drag path
- **WHEN** drag-and-drop is unavailable or not used
- **THEN** the context-menu move actions SHALL provide equivalent category assignment behavior

### Requirement: Session export entry point
The session card context menu SHALL provide an export action.

#### Scenario: Open export action
- **WHEN** the user chooses Export from a session card context menu
- **THEN** the workspace SHALL let the user choose JSON or Markdown format and request export through the frontend service

#### Scenario: Export feedback
- **WHEN** export completes or fails
- **THEN** the workspace SHALL show localized feedback without blocking unrelated session navigation

### Requirement: CLI-specific session icons
The workspace shell SHALL render CLI-specific visual identity for sessions based on each session's stable agent id.

#### Scenario: Render session card CLI icon
- **WHEN** a session card renders for `claude-code`, `codex-cli`, `gemini-cli`, or `opencode`
- **THEN** the card SHALL show the corresponding CLI icon or semantic icon treatment for that stable agent id
- **AND** the icon SHALL remain visually distinct from the other managed CLI tools

#### Scenario: Render created session with selected CLI icon
- **WHEN** the user creates a session from the create-session dialog for a selected CLI
- **THEN** the created session SHALL appear in workspace navigation with that selected CLI's icon identity

#### Scenario: Fallback unknown agent icon
- **WHEN** a session references an unknown or future agent id
- **THEN** the workspace SHALL render a neutral fallback agent icon without failing the session list

#### Scenario: Preserve compact session layout
- **WHEN** CLI-specific icons render in session cards, active-session headers, or session-adjacent workspace surfaces
- **THEN** long titles, folder paths, status markers, and context actions SHALL not overlap the icon or each other

### Requirement: Session context menu pointer positioning

The main session context menu SHALL open near the user's right-click pointer and remain inside the visible viewport.

#### Scenario: Right-click lower sessions

- **WHEN** the user opens the context menu on any visible session row
- **THEN** the menu SHALL appear near the pointer position
- **AND** it SHALL NOT drift to the top of the page solely because the row is lower in the sidebar.

#### Scenario: Menu reaches viewport edge

- **WHEN** the preferred pointer-adjacent menu position would overflow the viewport
- **THEN** the menu SHALL flip or clamp using its measured rendered dimensions.

