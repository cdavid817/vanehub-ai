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
