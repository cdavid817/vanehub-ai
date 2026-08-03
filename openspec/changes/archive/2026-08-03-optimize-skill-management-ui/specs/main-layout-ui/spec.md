## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Optimized information panel tabs
The information panel SHALL provide keep-alive tabs for Basic Info, Token Usage, and Skill, and the Skill tab SHALL provide keep-alive Effective, Global, and Project subviews derived from the active session context.

#### Scenario: Information panel tab set
- **WHEN** the information panel renders for an active session
- **THEN** the panel SHALL show tabs named Basic Info, Token Usage, and Skill
- **AND** the panel SHALL NOT show Files, Changes, or Logs tabs in the compact information panel

#### Scenario: Switch tabs without unmounting content
- **WHEN** the user switches between information panel tabs
- **THEN** all tab contents SHALL remain mounted while only the selected tab content is visible

#### Scenario: Show selected session model
- **WHEN** the Basic Info tab is visible for an active session
- **THEN** the tab SHALL show the active CLI identity, session lifecycle state, project or worktree context, and the model id from that session's chat configuration
- **AND** it SHALL show a localized empty state when no model id is available

#### Scenario: Show session token usage
- **WHEN** the Token Usage tab is visible for an active session
- **THEN** the tab SHALL show reported input, output, cache-read, cache-creation, and total token counts for that session when reported usage exists
- **AND** it SHALL keep estimated character activity separate from reported token totals

#### Scenario: Show no reported token fallback
- **WHEN** the Token Usage tab is visible and the active session has no reported token totals
- **THEN** the tab SHALL show a localized no-reported-token state
- **AND** it SHALL include estimated response and character context when estimated usage exists

#### Scenario: Show Skill scope subviews
- **WHEN** the Skill tab is visible for an active session
- **THEN** it SHALL show Effective, Global, and Project subviews with localized counts
- **AND** switching those subviews SHALL preserve their loaded content and local UI state

#### Scenario: Show effective Skills
- **WHEN** the Effective Skill subview is visible
- **THEN** it SHALL show enabled global and project Skills applicable to the active stable Agent id
- **AND** each Skill SHALL retain a visible global or project scope label so same-id Skills remain distinguishable

#### Scenario: Show global Skills read-only
- **WHEN** the Global Skill subview is visible
- **THEN** it SHALL show global Skills assigned to the active Agent with enablement and binding status
- **AND** SHALL keep global Skill mutations out of the information panel
- **AND** SHALL provide navigation to the global Skill Settings page

#### Scenario: Show complete project Skill inventory
- **WHEN** the Project Skill subview is visible with a resolved workspace path
- **THEN** it SHALL show all project Skills for that workspace, including disabled, unbound, and drifted Skills
- **AND** disabled or paused Skills SHALL NOT appear in the Effective subview

#### Scenario: Localize optimized information panel
- **WHEN** the optimized information panel renders in any supported application locale
- **THEN** all user-visible labels, tab names, Skill subview names, actions, loading states, empty states, errors, confirmations, and section headings SHALL use the active locale resources
- **AND** stable Agent ids, model ids, project paths, worktree names, and Skill ids MAY remain literal identifiers

#### Scenario: Preserve compact panel behavior
- **WHEN** the optimized information panel renders in `futuristic` or `minimal` style
- **THEN** it SHALL use shared semantic panel, muted-panel, segmented-control, border, text, and status tokens
- **AND** long labels, model ids, paths, Skill names, and project Skill controls SHALL not overlap adjacent controls or resize the workspace grid
- **AND** complex project Skill forms and confirmations SHALL render in application-level dialogs rather than expanding the compact panel width
