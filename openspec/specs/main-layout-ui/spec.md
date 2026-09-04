# main-layout-ui Specification

## Purpose
Defines the workspace shell layout, sidebar session organization, main content sizing, collapsible information panel behavior, keep-alive panel tabs, and internal scrolling rules shared by the Tauri desktop frontend and browser Web runtime.
## Requirements
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

#### Scenario: Keep top bar search reachable at responsive widths
- **WHEN** the workspace width is at or below 900px
- **THEN** the top bar SHALL keep the Agent/session/task search reachable, either rendered directly or through an equivalent accessible icon-triggered control
- **AND** search SHALL NOT be removed from the top bar without a replacement control

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

#### Scenario: Render expanded panel layout
- **WHEN** the session navigation and Inspector are both open at wide-layout width
- **THEN** the workspace SHALL render navigation, main surface, and Inspector side by side within their documented bounded widths
- **AND** the main surface SHALL receive the remaining width without overlap

#### Scenario: Render collapsed information panel layout
- **WHEN** the session navigation is open and the Inspector is closed
- **THEN** the main surface SHALL expand into the space the Inspector would otherwise occupy

#### Scenario: Render collapsed session sidebar layout
- **WHEN** the session navigation is closed and the Inspector is open
- **THEN** the main surface SHALL expand into the space the session navigation would otherwise occupy

#### Scenario: Render both panels collapsed
- **WHEN** the session navigation and Inspector are both closed
- **THEN** the main surface SHALL occupy the full available width

#### Scenario: Align panel bottoms
- **WHEN** the workspace shell renders between the top bar and status bar
- **THEN** the activity rail and all visible workspace panels SHALL use the same available height and align at the bottom edge

#### Scenario: Separate the session list from the conversation surface
- **WHEN** the session navigation and main conversation surface are both visible
- **THEN** the workspace SHALL reserve a non-overlapping visual gap between the two surfaces
- **AND** the gap SHALL NOT reduce or cover the session cards' trailing content
- **AND** the gap SHALL be removed when session navigation is closed or the workspace switches to its narrow single-surface composition

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

#### Scenario: Omit sidebar utility row
- **WHEN** the session navigation pane renders
- **THEN** it SHALL omit Settings, Help, and any visual-style-switching control, because global utility actions belong to the activity rail's utility group

#### Scenario: Show agent marker on session cards
- **WHEN** a session row is rendered
- **THEN** the row SHALL show an agent-type marker using a distinct icon and color for every registered stable agent id, including Claude Code, Codex CLI, OpenCode, Gemini CLI, Antigravity CLI, and OnePiece
- **AND** an unrecognized future agent id SHALL render a neutral fallback marker rather than failing the row

#### Scenario: Open create-session dialog from new action
- **WHEN** the user activates the New Session action
- **THEN** the pane SHALL open the create-session wizard rather than immediately creating a session

#### Scenario: Create session from dialog
- **WHEN** the user completes the create-session wizard
- **THEN** the UI SHALL create a session through the frontend agent service and make the created session available for selection

#### Scenario: Select session card
- **WHEN** the user selects a session row
- **THEN** the pane SHALL switch the active session through the frontend agent service and visually mark that row as selected

#### Scenario: Switch to activity view
- **WHEN** the user is viewing an alternate organization and chooses to return to the default view
- **THEN** the pane SHALL restore the attention-first grouping described in "Render the default view"

#### Scenario: Sort activity groups by priority
- **WHEN** the pane renders the default attention-first view
- **THEN** groups SHALL appear in priority order: needs-input, pending verification or approval, running, pinned, recent, remaining

#### Scenario: Show pinned sessions
- **WHEN** one or more sessions are pinned
- **THEN** the pane SHALL render pinned sessions in their own group within the attention-first ordering, ranked before recent and remaining sessions

#### Scenario: Switch to folder group view
- **WHEN** the user selects category organization
- **THEN** the pane SHALL group sessions by their assigned category, including a localized uncategorized group for sessions without one

#### Scenario: Toggle folder expansion
- **WHEN** the user toggles a category group in category organization
- **THEN** the pane SHALL expand or collapse that category's session rows without changing the selected session

#### Scenario: Open archived view
- **WHEN** the user selects archived organization
- **THEN** the pane SHALL show archived sessions from the frontend agent service and indicate the archived session count

#### Scenario: Use context actions
- **WHEN** the user opens a session row's context menu
- **THEN** the pane SHALL provide actions to rename, pin or unpin, archive or restore, and delete the session according to its current state

#### Scenario: Prevent browser context menu
- **WHEN** the user opens the custom session context menu in browser or desktop WebView mode
- **THEN** the pane SHALL prevent the platform default context menu from appearing over the custom menu

#### Scenario: Confirm destructive session deletion
- **WHEN** the user chooses to delete a session
- **THEN** the pane SHALL ask for confirmation before calling the delete operation

#### Scenario: Scroll long session lists internally
- **WHEN** the session list content exceeds the pane height
- **THEN** the list SHALL scroll inside the pane, using virtualization per "Render a large history" once the visible source is large, without scrolling the whole workspace shell

#### Scenario: Compact secondary session controls
- **WHEN** the pane renders batch-management, organization-switching, filter, and archived-view entry points alongside the session list
- **THEN** these secondary controls SHALL be exposed through the shared bounded toolbar or overflow described in "Keep controls compact" rather than each occupying its own dedicated full-width row
- **AND** the session list SHALL remain the dominant vertical element of the pane

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

#### Scenario: Select session mode
- **WHEN** the wizard's runtime-mode step opens
- **THEN** it SHALL present Single Agent and Multi Agent as selectable modes, alongside the CLI/API and Local/Remote choices described in "Choose runtime mode"
- **AND** only combinations the current service capabilities do not support SHALL be disabled, each with an explanation

#### Scenario: Multi Agent is disabled
- **WHEN** a specific runtime-mode combination is not supported by current service capabilities
- **THEN** that combination SHALL be marked disabled with an explanation
- **AND** the user SHALL NOT be able to submit an unsupported combination
- **AND** Multi Agent itself SHALL NOT be universally disabled, since multi-participant sessions are a supported capability

#### Scenario: Select Agent
- **WHEN** the participant step renders
- **THEN** the wizard SHALL let the user choose among every registered stable agent id, including Claude Code, Gemini CLI, Codex CLI, OpenCode, Antigravity CLI, and OnePiece, subject to the selected runtime mode
- **AND** Multi Agent mode SHALL let the user configure more than one participant per "Configure participants"

#### Scenario: Show project history
- **WHEN** the workspace step renders
- **THEN** it SHALL show recently selected project folders and remote workspaces from the frontend agent service

#### Scenario: Browse project folder
- **WHEN** the user chooses to browse for a project folder
- **THEN** the workspace step SHALL request folder selection through the frontend agent service

#### Scenario: Show worktree controls for Git project
- **WHEN** the selected project folder is a Git repository
- **THEN** the workspace step SHALL show an optional worktree checkbox and a worktree name field when the checkbox is enabled

#### Scenario: Disable worktree controls for non-Git project
- **WHEN** the selected project folder is not a Git repository
- **THEN** the workspace step SHALL allow normal session creation and SHALL hide or disable worktree controls

#### Scenario: Submit concise failures
- **WHEN** project inspection, folder selection, or session creation fails
- **THEN** the wizard SHALL show a concise error message without rendering raw stdout or stderr, per "Show validation errors"
- **AND** the message SHALL remain fully readable rather than being truncated to a single line
- **AND** it SHALL be announced to assistive technology

#### Scenario: Dismiss without creating
- **WHEN** the wizard is open and no creation request is in flight
- **THEN** pressing Escape SHALL close it without creating a session
- **AND** focus SHALL return to the control that opened it

#### Scenario: Dismissal blocked while creating
- **WHEN** a creation request is in flight
- **THEN** Escape and backdrop dismissal SHALL NOT close the wizard, per "Submit once"

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
- **WHEN** a session card renders for `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, or `antigravity-cli`
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

#### Scenario: Information panel tab set
- **WHEN** the Inspector renders Session Overview for an active session
- **THEN** it SHALL show Participant, Runtime, Usage, Skill, Workspace, IM, and Code Index as bounded sections rather than equal-width tabs
- **AND** it SHALL NOT show Files, Changes, or Logs sections in Overview mode; that evidence remains reachable through their owning primary or Runtime Panel surfaces

#### Scenario: Switch tabs without unmounting content
- **WHEN** the user scrolls between Overview sections or returns to Overview after Follow Selection or Pinned mode
- **THEN** each section's already-loaded data SHALL remain available without an unnecessary reload
- **AND** an unexpanded or not-yet-visible section MAY defer its own query until needed, per "Protect panel performance"

#### Scenario: Show selected session model
- **WHEN** the Runtime section is visible for an active session
- **THEN** it SHALL show the active CLI identity, session lifecycle state, project or worktree context, and the model id from that session's chat configuration
- **AND** it SHALL show a localized empty state when no model id is available

#### Scenario: Show session token usage
- **WHEN** the Usage section is visible for an active session
- **THEN** it SHALL show reported input, output, cache-read, cache-creation, and total token counts for that session when reported usage exists
- **AND** it SHALL keep estimated character activity separate from reported token totals

#### Scenario: Show no reported token fallback
- **WHEN** the Usage section is visible and the active session has no reported token totals
- **THEN** it SHALL show a localized no-reported-token state
- **AND** it SHALL include estimated response and character context when estimated usage exists

#### Scenario: Show Skill scope subviews
- **WHEN** the Skill section is visible for an active session
- **THEN** it SHALL show Effective, Global, and Project subviews with localized counts
- **AND** switching those subviews SHALL preserve their loaded content and local UI state

#### Scenario: Show effective Skills
- **WHEN** the Effective Skill subview is visible
- **THEN** it SHALL show enabled global and project Skills applicable to the active stable Agent id
- **AND** each Skill SHALL retain a visible global or project scope label so same-id Skills remain distinguishable

#### Scenario: Show global Skills read-only
- **WHEN** the Global Skill subview is visible
- **THEN** it SHALL show global Skills assigned to the active Agent with enablement and binding status
- **AND** SHALL keep global Skill mutations out of the Inspector
- **AND** SHALL provide navigation to the global Skill Settings page via EvidenceLink

#### Scenario: Show complete project Skill inventory
- **WHEN** the Project Skill subview is visible with a resolved workspace path
- **THEN** it SHALL show all project Skills for that workspace, including disabled, unbound, and drifted Skills
- **AND** disabled or paused Skills SHALL NOT appear in the Effective subview

#### Scenario: Localize optimized information panel
- **WHEN** the Inspector renders in any registered application locale
- **THEN** all user-visible labels, section names, Skill subview names, actions, loading states, empty states, errors, confirmations, and headings SHALL use the active locale resources
- **AND** stable Agent ids, model ids, project paths, worktree names, and Skill ids MAY remain literal identifiers

#### Scenario: Preserve compact panel behavior
- **WHEN** the Inspector renders inline or as a sheet in `futuristic` or `minimal` style
- **THEN** it SHALL use shared semantic panel, muted-panel, segmented-control, border, text, and status tokens
- **AND** long labels, model ids, paths, Skill names, and project Skill controls SHALL not overlap adjacent controls or resize the workspace grid
- **AND** complex project Skill forms and confirmations SHALL render in application-level dialogs rather than expanding the Inspector's width

### Requirement: Session batch management mode
The workspace session sidebar SHALL provide an explicit batch-management mode for selecting multiple visible sessions and running a confirmed delete operation.

#### Scenario: Enter batch management
- **WHEN** the user activates the batch-management action from the session sidebar
- **THEN** the sidebar SHALL show selectable controls on visible session rows
- **AND** it SHALL show localized selected-count, select-visible, delete-selected, and exit-batch controls

#### Scenario: Toggle session selection
- **WHEN** batch-management mode is active and the user selects a session row checkbox or row selection affordance
- **THEN** the sidebar SHALL toggle that session id in the batch selection
- **AND** it SHALL NOT switch the active session as part of that toggle

#### Scenario: Select visible sessions
- **WHEN** batch-management mode is active and the user activates select-visible
- **THEN** the sidebar SHALL select every currently visible session in the active search, Agent filter, archive, and presentation state
- **AND** it SHALL update the selected-count control

#### Scenario: Confirm batch deletion
- **WHEN** batch-management mode is active and one or more sessions are selected
- **AND** the user activates delete-selected
- **THEN** the sidebar SHALL show a localized destructive confirmation that includes the number of selected sessions
- **AND** it SHALL call session deletion only after the user confirms

#### Scenario: Exit batch management
- **WHEN** the user exits batch-management mode
- **THEN** the sidebar SHALL hide selectable controls
- **AND** it SHALL clear the current batch selection
- **AND** normal session selection, context menu, and category drag behavior SHALL be restored

### Requirement: Session list presentation switch
The workspace session sidebar SHALL let users switch between a flat list presentation and a categorized presentation.

#### Scenario: Use list presentation
- **WHEN** the list presentation is selected
- **THEN** the sidebar SHALL render matching sessions as a flat scannable list while preserving pinned and archived indicators

#### Scenario: Use categorized presentation
- **WHEN** the categorized presentation is selected
- **THEN** the sidebar SHALL render matching sessions grouped by their user-defined category
- **AND** it SHALL include a localized uncategorized group for sessions without a category

#### Scenario: Preserve presentation while filtering
- **WHEN** search text or Agent filter changes
- **THEN** the sidebar SHALL keep the selected presentation mode
- **AND** it SHALL apply the new filter within that presentation

### Requirement: Session Agent filter
The workspace session sidebar SHALL provide an Agent filter for All, Claude Code, OpenCode, Codex CLI, and Gemini CLI sessions.

#### Scenario: Filter all sessions
- **WHEN** the user selects the All Agent filter
- **THEN** the sidebar SHALL include sessions for every Agent in the active session source

#### Scenario: Filter by managed CLI Agent
- **WHEN** the user selects Claude Code, OpenCode, Codex CLI, or Gemini CLI
- **THEN** the sidebar SHALL show only sessions whose stable `agentId` matches the selected managed CLI Agent id
- **AND** it SHALL NOT match by display name

#### Scenario: Filter archived sessions
- **WHEN** the archived session source is visible and an Agent filter is active
- **THEN** the sidebar SHALL filter archived sessions by the same stable Agent id semantics

### Requirement: Session management visual and localization parity
The optimized session management controls SHALL remain consistent with the workspace visual design system and synchronized zh-CN/en localization.

#### Scenario: Render localized session management controls
- **WHEN** the sidebar renders batch-management, presentation, Agent filter, and confirmation controls
- **THEN** every user-visible label, tooltip, accessible name, empty state, and destructive confirmation SHALL use the active locale

#### Scenario: Preserve visual styles
- **WHEN** the workspace renders in `futuristic` or `minimal` style
- **THEN** the optimized session management controls SHALL use existing semantic tokens, compact dimensions, stable spacing, and lucide or project icons without overlapping adjacent text

### Requirement: Resizable session sidebar
The workspace shell SHALL let users resize the expanded session sidebar horizontally within bounded minimum and maximum widths.

#### Scenario: Drag sidebar resize handle
- **WHEN** the session sidebar is expanded and the user drags its resize handle horizontally
- **THEN** the sidebar width SHALL update within bounded limits
- **AND** the main content SHALL resize without overlapping the activity bar, information panel, or status bar

#### Scenario: Persist sidebar width preference
- **WHEN** the user changes the session sidebar width
- **THEN** the workspace SHALL remember the width preference for later workspace renders in the same browser or desktop WebView profile

#### Scenario: Collapse preserves width preference
- **WHEN** the user collapses and re-expands the session sidebar after resizing it
- **THEN** the sidebar SHALL restore the last bounded expanded width
- **AND** hidden sidebar controls SHALL remain unreachable while collapsed

### Requirement: Project-grouped session sidebar
The workspace session sidebar SHALL provide a project grouping presentation that groups sessions by their owning worktree, project, folder, or remote workspace metadata.

#### Scenario: Render project groups
- **WHEN** project grouping is selected
- **THEN** the sidebar SHALL render sessions under project groups derived from service-backed session metadata
- **AND** each project group SHALL show a concise label, a session count, and an expand/collapse control

#### Scenario: Toggle project group expansion
- **WHEN** the user toggles a project group
- **THEN** the sidebar SHALL expand or collapse that project's session cards without changing the active session

#### Scenario: Ungrouped project bucket
- **WHEN** visible sessions have no project, folder, worktree, or remote workspace metadata
- **THEN** the sidebar SHALL render those sessions in a localized ungrouped project bucket

#### Scenario: Preserve filtering and archived behavior
- **WHEN** search, Agent filtering, archived view, pinned sessions, or batch-management mode is active
- **THEN** project grouping SHALL apply to the currently visible session source without bypassing existing selection and context actions

### Requirement: Lazy Loop Center loading
The workspace shell SHALL dynamically load the Loop Center task-board module on first activation while preserving mounted session workspace state.

#### Scenario: Use sessions without opening Loops
- **WHEN** the user operates the session workspace without selecting the Loops activity
- **THEN** the Loop Center module SHALL remain unloaded

#### Scenario: Open Loops for the first time
- **WHEN** the user selects the Loops activity before its module has loaded
- **THEN** the main content region SHALL show a localized loading state until Loop Center is available
- **AND** the session workspace SHALL retain its selected session and mounted tab state

#### Scenario: Return to a loaded Loop Center
- **WHEN** the user returns to Loops after its module has loaded
- **THEN** the Loop Center SHALL render without resetting its task-board state

#### Scenario: Fail to load Loop Center
- **WHEN** the Loop Center module load fails
- **THEN** the main content region SHALL provide a localized retry action
- **AND** the user SHALL still be able to return to the session workspace

### Requirement: Session-context project Skill management
The information panel Project Skill subview SHALL provide complete project Skill management for the active session workspace through the frontend service boundary while keeping complex forms and destructive confirmations outside the compact panel layout.

#### Scenario: Resolve active project Skill scope
- **WHEN** the Project Skill subview loads for an active session
- **THEN** it SHALL use `worktreePath` when present and otherwise use `projectPath` as the workspace Skill path
- **AND** SHALL display the normalized resolved path without allowing it to resize the workspace grid

#### Scenario: No active project context
- **WHEN** an active session has neither a worktree path nor a project path
- **THEN** the Project Skill subview SHALL show a localized no-project state
- **AND** SHALL NOT offer a manual workspace path field or project Skill mutations

#### Scenario: Manage project Skill lifecycle
- **WHEN** an active session has a resolved workspace path
- **THEN** the Project Skill subview SHALL allow users to create, import, preview, edit, enable or disable, and delete Skills in that workspace scope
- **AND** SHALL use application-level dialogs for forms, Markdown preview, stale-edit recovery, and destructive confirmation

#### Scenario: Bind project Skill to active CLI Agent
- **WHEN** the active session Agent is CLI-capable and the user assigns or removes a project Skill
- **THEN** the panel SHALL call the granular CLI bind or unbind operation with the active stable Agent id and resolved workspace scope
- **AND** SHALL describe confirmed active bindings as mounts only when binding data reports them mounted

#### Scenario: Bind project Skill to active API Agent
- **WHEN** the active session Agent is API-kind and the user assigns or removes a project Skill
- **THEN** the panel SHALL call the granular API bind or unbind operation with the active stable Agent id and resolved workspace scope
- **AND** SHALL describe the relationship as prompt injection without showing filesystem mount terminology

#### Scenario: Preserve disabled project assignment
- **WHEN** a project Skill is disabled while retaining an active-session Agent binding
- **THEN** the panel SHALL identify the binding as configured but paused
- **AND** SHALL NOT identify it as an active mount or effective Skill

#### Scenario: Show and synchronize project drift
- **WHEN** the project Skill overview reports source, registry, or CLI mount drift
- **THEN** the Project subview SHALL show an actionable issue summary and synchronization control
- **AND** SHALL keep backup, overwrite, restored, and failed synchronization results reviewable

#### Scenario: Keep project operations service-backed
- **WHEN** the information panel performs a project Skill query or mutation in the Tauri desktop or Web/mock runtime
- **THEN** the React component SHALL call the frontend Skill service boundary
- **AND** SHALL NOT call Tauri `invoke()` or access the filesystem directly

### Requirement: Multi-Agent session presence surfaces
The workspace shell SHALL represent a multi-Agent session with bounded metadata in session navigation and a dedicated member-information card in the Basic information panel instead of duplicating the roster in the conversation header.

#### Scenario: Render a multi-Agent session card
- **WHEN** a session card represents a session with more than one active participant
- **THEN** the card SHALL keep the primary CLI's native brand icon
- **AND** the card SHALL show a localized multi-Agent label in its bounded metadata row
- **AND** long titles, project metadata, state markers, and context actions SHALL remain unobstructed
- **AND** the session navigation SHALL NOT introduce a horizontal scrollbar

#### Scenario: Render member information in the information panel
- **WHEN** the information panel displays a multi-Agent session
- **THEN** it SHALL show a visually independent member-information card listing active participants with role, Agent, model family, and state
- **AND** it SHALL provide service-backed add and leave actions
- **AND** it SHALL NOT offer an action that chooses the next speaker

#### Scenario: Separate member information from basic information
- **WHEN** the information panel displays a multi-Agent session
- **THEN** Member Information SHALL be an independent peer tab beside Basic Info, Token Usage, and Skill
- **AND** selecting Member Information SHALL display the membership card without mixing it into the Basic Info pane
- **AND** the four labels SHALL remain bounded within the information-panel width

#### Scenario: Hide member-information card for a single-Agent session
- **WHEN** the information panel displays a session that has never held more than one participant
- **THEN** the member-information card SHALL NOT be rendered
- **AND** the Member Information tab SHALL NOT be rendered

#### Scenario: Render departed participant history
- **WHEN** the session has departed participants
- **THEN** the Member Information tab SHALL make their historical participation reviewable without presenting them as active routing targets

#### Scenario: Preserve single-Agent layout
- **WHEN** a session has one active participant and no departed participants
- **THEN** session navigation and Basic information SHALL preserve the existing compact single-Agent presentation

#### Scenario: Localize roster controls
- **WHEN** roster presence or membership controls render in any supported locale
- **THEN** all labels, statuses, actions, conflicts, and accessible names SHALL use synchronized locale resources

### Requirement: Reversible conversation focus mode
The workspace shell SHALL provide an explicit focus mode that gives the conversation surface the available width without discarding navigation or information-panel state.

#### Scenario: Enter conversation focus mode
- **WHEN** the user activates conversation focus mode from the workspace header
- **THEN** the session sidebar and information panel SHALL collapse together
- **AND** the global header SHALL contract to a compact escape surface and the session workspace tab bar SHALL collapse
- **AND** the conversation surface SHALL expand into the released width
- **AND** the focus-mode control SHALL remain visible with a localized accessible name

#### Scenario: Exit conversation focus mode
- **WHEN** the user exits conversation focus mode
- **THEN** the session sidebar and information panel SHALL return to the expanded or collapsed states they held before focus mode was entered
- **AND** the active session, selected workspace tab, messages, and composer draft SHALL remain unchanged

### Requirement: Role-semantic participant icons
Multi-Agent presence surfaces SHALL distinguish participant responsibilities visually while retaining Agent identity in text.

#### Scenario: Render built-in role icons
- **WHEN** a participant holds the built-in architect, implementer, or reviewer role
- **THEN** roster presence SHALL render a distinct semantic icon for that role
- **AND** the adjacent label SHALL continue to name both the role and Agent

#### Scenario: Render a built-in role without a captured snapshot
- **WHEN** a persisted participant carries a stable built-in role id but no role snapshot
- **THEN** the member-information card SHALL show the built-in role name as its primary label
- **AND** it SHALL show the stable CLI id as secondary information without duplicating that id as the role label

#### Scenario: Render custom or unassigned roles
- **WHEN** a participant holds a custom role with an avatar
- **THEN** roster presence SHALL render the captured custom avatar
- **AND** when no role presentation exists it SHALL fall back to the Agent brand icon

### Requirement: Conversation-first workspace height
The workspace shell SHALL avoid persistent decorative chrome that reduces the conversation height without exposing actionable state.

#### Scenario: Render the workspace without a bottom status strip
- **WHEN** the main workspace is displayed
- **THEN** it SHALL NOT render the decorative bottom status strip
- **AND** runtime and turn state SHALL remain available in contextual session and conversation surfaces

### Requirement: Contiguous desktop chat workspace
The desktop session workspace SHALL prioritize conversation content with adjacent regions and quiet separators instead of presenting every region as an independent floating card.

#### Scenario: Render the expanded desktop workspace
- **WHEN** session navigation, conversation, and information regions are expanded on a desktop viewport
- **THEN** the regions SHALL share the available height and meet at visible one-pixel boundaries without decorative outer gutters
- **AND** repeated panel rounding, heavy shadow, background grid texture, and glass blur SHALL NOT compete with conversation content
- **AND** the existing session-list resize and overflow visibility controls SHALL remain operable

#### Scenario: Render dense session navigation
- **WHEN** the session list contains active, pinned, or archived sessions
- **THEN** each row SHALL preserve CLI identity, title, state, date, and multi-Agent metadata in a compact two-line hierarchy
- **AND** the selected row SHALL remain distinguishable without relying on color alone
- **AND** search, grouping, filtering, batch actions, and keyboard focus SHALL remain available

#### Scenario: Keep workspace boundaries visible
- **WHEN** the desktop workspace is expanded, focused, or independently collapsed
- **THEN** the session-list-to-conversation divider SHALL remain visible in light and dark themes
- **AND** the workspace SHALL retain a continuous bottom boundary without clipped segments

#### Scenario: Draw one window-wide bottom divider
- **WHEN** any workspace destination or panel-collapse combination fills the application window
- **THEN** one top-layer semantic divider SHALL span the complete workspace width at the bottom edge
- **AND** activity-bar, navigation, conversation, information-panel, and scrollable backgrounds SHALL NOT cover or fragment that divider

### Requirement: Session overflow visibility controls
The conversation header SHALL provide a familiar overflow entry point for secondary workspace visibility actions.

#### Scenario: Toggle workspace regions from the overflow menu
- **WHEN** the user opens the conversation overflow menu
- **THEN** it SHALL expose accessible controls for the session list, information panel, and workspace tab row
- **AND** every control SHALL communicate its expanded state
- **AND** toggling one region SHALL preserve the selected session, selected workspace tab, messages, and composer draft

#### Scenario: Use one information-panel visibility entry point
- **WHEN** the conversation overflow menu is available
- **THEN** information-panel visibility SHALL be controlled from that menu
- **AND** the information panel SHALL NOT render separate collapse or edge-mounted expand buttons

#### Scenario: Hide empty terminal history badge
- **WHEN** the terminal-history count is zero
- **THEN** the terminal-history tab SHALL omit its numeric badge
- **AND** a positive count SHALL remain visible

### Requirement: Workspace bottom boundary
The workspace shell SHALL render a continuous one-pixel, theme-aware divider at its bottom edge so the application content remains visually distinct from the operating-system taskbar or adjacent desktop surface.

#### Scenario: Render workspace above the operating-system taskbar
- **WHEN** the main workspace is visible in desktop or Web mode
- **THEN** a non-interactive divider SHALL span the complete workspace width at the bottom edge
- **AND** the divider SHALL use the active theme's border semantics without reducing usable content height

### Requirement: Responsive session selection
The workspace SHALL reflect selection of an already-loaded, non-archived session without waiting for active-session persistence to complete, while the frontend agent service remains the authoritative persistence boundary.

#### Scenario: Select an already-loaded session
- **WHEN** the user selects a different non-archived session card that is already present in the sidebar
- **THEN** the selected marker and workspace SHALL begin rendering that session immediately
- **AND** persistence SHALL continue asynchronously through the frontend agent service

#### Scenario: Session persistence fails
- **WHEN** persisting an optimistic session selection fails
- **THEN** the workspace SHALL restore the previously active session
- **AND** the user SHALL receive an error notification

#### Scenario: Rapid successive selection
- **WHEN** the user selects multiple sessions before earlier persistence requests finish
- **THEN** the most recently selected session SHALL remain visible
- **AND** a late result from an older request SHALL NOT replace or roll back the newer selection

#### Scenario: Select the current session
- **WHEN** the user selects the session that is already active
- **THEN** the workspace SHALL avoid resetting session-scoped tabs, drafts, or message subscriptions

#### Scenario: Revisit a recently displayed session
- **WHEN** the user returns to a session whose conversation data remains cached
- **THEN** the workspace SHALL render the cached conversation immediately while any required refresh continues in the background

### Requirement: Todo Board workspace destination
The workspace SHALL expose Todo Board as a first-class full-screen activity destination alongside Sessions, Plans, and Loops.

#### Scenario: Open Todo Board
- **WHEN** the user activates the Todo Board activity entry
- **THEN** the workspace SHALL mark that entry active and render the board in the primary workspace region
- **AND** it SHALL preserve the existing Session, Plan, and Loop destination state for later return

### Requirement: Workspace auxiliary dialog behavior
The scheduled-tasks dialog and the session batch-delete confirmation SHALL obtain dismissal, focus containment, and focus return from the shared application dialog primitive.

#### Scenario: Dismiss the scheduled-tasks dialog
- **WHEN** the scheduled-tasks dialog is open
- **THEN** pressing Escape SHALL close it and focus SHALL return to the activity bar control that opened it

#### Scenario: Confirm destructive batch deletion
- **WHEN** the batch-delete confirmation is open
- **THEN** focus SHALL be placed inside the confirmation and SHALL remain contained within it
- **AND** Escape SHALL cancel the deletion while a delete request is not in flight

### Requirement: In-application session category creation
Creating a session category from the session context menu SHALL use an in-application dialog.

#### Scenario: Create and assign in one step
- **WHEN** the user chooses to create a category for a session and submits a non-empty name
- **THEN** the system SHALL create the category, assign that session to it, and close the dialog

#### Scenario: Empty or rejected name
- **WHEN** the submitted category name is empty or the creation request fails
- **THEN** the dialog SHALL stay open and SHALL present the reason next to the field

### Requirement: Routed workspace destinations
Activating a workspace destination SHALL change the route, and destination state SHALL survive that navigation. The workspace SHALL NOT expose a standalone Plans destination; planning SHALL remain within an eligible OnePiece session.

#### Scenario: Destination activation navigates
- **WHEN** the user activates the Sessions, Loops, or Todo Board activity entry
- **THEN** the workspace SHALL navigate to that destination's route
- **AND** the activity bar SHALL mark the entry active from the route

#### Scenario: Mounted destination state survives navigation
- **WHEN** the user leaves a visited destination and later returns to it
- **THEN** that destination SHALL retain the state it had when it was left
- **AND** it SHALL NOT be remounted or refetched solely because the route changed

#### Scenario: Return from a cross-destination jump
- **WHEN** the user opens a session from the Loop Center and then triggers Back
- **THEN** the workspace SHALL return to the destination the jump started from

#### Scenario: Unknown destination segment
- **WHEN** a workspace route names a destination that does not exist or names the retired Plans destination
- **THEN** the workspace SHALL fall back to the Sessions destination rather than rendering an empty region

### Requirement: OnePiece owns the planning surface
The workspace SHALL expose Plan mode only within the conversation bar of an eligible OnePiece session and SHALL NOT expose Plan draft, PlanRun, or Plan execution controls in the left activity bar or another global workspace destination.

#### Scenario: Open planning controls
- **WHEN** the active session uses the stable agent id `onepiece`
- **THEN** its conversation bar SHALL expose the available session execution modes including Plan mode
- **AND** the user SHALL remain on the session route while selecting or using Plan mode

#### Scenario: Render the activity bar
- **WHEN** the workspace activity bar renders in the desktop or Web runtime
- **THEN** it SHALL NOT render a Plans or Plan execution entry

### Requirement: Addressable session creation
Opening the create-session dialog SHALL be expressible as a route.

#### Scenario: External trigger opens creation
- **WHEN** the floating assistant or another external surface requests a new session
- **THEN** the workspace SHALL navigate to the session-creation route and open the dialog

#### Scenario: Dismissing creation leaves the route
- **WHEN** the user closes the create-session dialog without creating a session
- **THEN** the workspace SHALL return to the destination route it came from

### Requirement: Mission Control workspace destination
The workspace activity bar SHALL expose a localized icon-only Mission Control destination in both Tauri and Web runtimes, preserve mounted Session workspace state while it is active, and remain operable at desktop and narrow widths.

#### Scenario: Open Mission Control
- **WHEN** the user activates the Mission Control activity entry
- **THEN** the workspace opens the Mission Control surface without a runtime-specific component branch
- **AND** preserves the selected Session and its mounted tab state for return

#### Scenario: Identify Mission Control entry
- **WHEN** the Mission Control icon-only entry is available
- **THEN** it provides localized accessible name, tooltip, focus, hover, and active states without shifting adjacent entries

### Requirement: Continuous transcript and composer surface
The workspace chat SHALL present the transcript, status area, runner controls, and message composer as one continuous panel, and SHALL NOT decorate the attached composer as a second nested conversation card.

#### Scenario: Render an attached composer
- **WHEN** a Session chat with a message composer is displayed
- **THEN** the transcript and composer SHALL share one outer surface and one theme-aware separator
- **AND** the composer SHALL NOT add a competing outer shadow, detached gap, or mixed square-and-rounded conversation frame

#### Scenario: Focus the message input
- **WHEN** keyboard focus enters the message input
- **THEN** the input controls SHALL expose a visible semantic focus state without changing the outer workspace geometry

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

#### Scenario: Render a seat-optional tab
- **WHEN** the Runtime Panel's Terminal History or Logs surface is active for a multi-Agent session
- **THEN** the workspace SHALL expose an all-seats choice and concrete active-seat choices according to the registry's seat-optional declaration
- **AND** the selected seat SHALL be included in that surface's service query key

#### Scenario: Render a seat-required tab
- **WHEN** the Runtime Panel's Shell surface is active for a multi-Agent session
- **THEN** the workspace SHALL require one concrete active seat before creating or attaching a Shell, per the registry's seat-required declaration

#### Scenario: Render a session-scoped tab
- **WHEN** the Changes, Files, or Report primary surface is active in its default mode
- **THEN** the workspace SHALL NOT show a global seat control that appears to filter that surface, per the registry's session-scoped declaration

#### Scenario: Add a future workspace tab
- **WHEN** a future primary or runtime surface is registered
- **THEN** its scope, retention, live-update, badge, and route-compatibility policies SHALL be declared in the same registry
- **AND** React SHALL NOT infer those semantics from its translated label or display order

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

