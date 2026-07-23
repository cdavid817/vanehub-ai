# session-workspace-tabs Specification

## Purpose
Defines the eight-tab session workspace, tab lifecycle, Chat composer behavior, execution history, document and report views, localization, themes, and Web preview behavior.
## Requirements
### Requirement: Eight-tab session workspace
The main content area SHALL provide session-scoped Chat, Changes, Documents, Files, Terminal, Shell, Logs, and Report tabs in that order.

#### Scenario: Render session tabs
- **WHEN** the main workspace is displayed
- **THEN** the tab bar SHALL show all eight tabs with a recognizable icon and localized label

#### Scenario: Default to Chat
- **WHEN** the workspace starts or a different session becomes active
- **THEN** Chat SHALL be the active tab

#### Scenario: Navigate tabs by keyboard
- **WHEN** keyboard focus is on the tab bar
- **THEN** the user SHALL be able to move between tabs and activate a tab using accessible tab-list keyboard behavior

#### Scenario: Fit tabs in a narrow panel
- **WHEN** the center panel cannot display all tab controls at once
- **THEN** the tab bar SHALL remain usable through internal horizontal scrolling without resizing the workspace shell

### Requirement: Lazy mount and keep-alive tab state
The session workspace SHALL mount a tab panel only after its first activation and SHALL keep visited panels mounted while the selected session remains unchanged.

#### Scenario: Activate an unvisited tab
- **WHEN** the user activates a tab that has not been visited for the selected session
- **THEN** the system SHALL mount that panel and add it to the mounted-tab set

#### Scenario: Return to a visited tab
- **WHEN** the user returns to a previously visited tab in the same session
- **THEN** the panel SHALL retain its component state and only its CSS visibility SHALL change

#### Scenario: Switch sessions
- **WHEN** the active session id changes
- **THEN** the system SHALL unmount old-session panels, reset the mounted-tab set to Chat, and activate Chat

#### Scenario: No active session
- **WHEN** no session is selected
- **THEN** Chat SHALL show the localized existing empty state and session-dependent tabs SHALL show a localized unavailable state without issuing native project or process operations

### Requirement: Chat-only composer
The session workspace SHALL display the existing chat composer only while Chat is active.

#### Scenario: Activate Chat
- **WHEN** the user activates Chat
- **THEN** the active session message list and composer SHALL be visible and retain existing send, stop, configuration, and streaming behavior

#### Scenario: Leave Chat
- **WHEN** the user activates any non-Chat tab
- **THEN** the composer SHALL be hidden without discarding the current chat draft

### Requirement: Terminal execution history
The Terminal tab SHALL present tool-use blocks from the selected session as status-aware execution cards.

#### Scenario: Count terminal entries
- **WHEN** selected-session messages contain tool-use blocks
- **THEN** the Terminal tab badge SHALL equal the number of tool-use blocks included in the loaded session history

#### Scenario: Render a tool-use card
- **WHEN** a tool-use block is displayed
- **THEN** the card SHALL show its tool name, localized status, structured input/output when present, and parent message time

#### Scenario: Partial terminal history
- **WHEN** the bounded message history does not cover the complete session
- **THEN** Terminal SHALL indicate that the displayed entries and badge are partial

#### Scenario: No terminal entries
- **WHEN** the selected session has no tool-use blocks
- **THEN** Terminal SHALL show a localized empty state

### Requirement: Session document viewer
The Documents tab SHALL list bounded Markdown and text documents under the selected session root and render selected content read-only.

#### Scenario: Render Markdown document
- **WHEN** the user selects a supported Markdown document
- **THEN** the tab SHALL render Markdown without executing raw embedded HTML

#### Scenario: Render text document
- **WHEN** the user selects a supported plain-text document
- **THEN** the tab SHALL preserve readable whitespace in a plain-text viewer

#### Scenario: Document scan truncated
- **WHEN** document discovery reaches its configured depth or count limit
- **THEN** the tab SHALL indicate that the document list is partial

#### Scenario: Document unavailable
- **WHEN** a document is oversized, binary, outside the session root, or no longer exists
- **THEN** the tab SHALL show a localized concise error without exposing raw native diagnostics

### Requirement: Session report
The Report tab SHALL summarize usage and activity from the selected session's bounded message history without conflating reported tokens with estimates.

#### Scenario: Show reported token distribution
- **WHEN** messages include reported input/output token usage
- **THEN** Report SHALL display input and output token totals as separately labelled values

#### Scenario: Show estimates separately
- **WHEN** completed assistant messages do not contain reported token usage
- **THEN** Report SHALL show estimated input/output character counts separately and SHALL NOT label them as tokens

#### Scenario: Show tool ranking and timeline
- **WHEN** the selected session contains messages or tool-use blocks
- **THEN** Report SHALL show tool frequency ranking and a localized chronological activity timeline

#### Scenario: Show status counts and completions
- **WHEN** the bounded history contains pending, streaming, completed, failed, or cancelled messages
- **THEN** Report SHALL show a count for each status and SHALL represent completed responses as localized completion events in the chronological timeline

#### Scenario: Partial report
- **WHEN** aggregation uses a bounded subset of session messages
- **THEN** Report SHALL visibly identify the report as partial

### Requirement: Localized and theme-aware session tabs
Every session tab SHALL use synchronized Simplified Chinese and English resources and SHALL remain readable in both registered visual styles.

#### Scenario: Change application language
- **WHEN** the user changes between Simplified Chinese and English
- **THEN** tab labels, badges, buttons, tooltips, accessibility labels, statuses, errors, empty states, dates, and numbers SHALL follow the active locale

#### Scenario: Use futuristic style
- **WHEN** `futuristic` style is active
- **THEN** all tab surfaces SHALL use existing semantic theme tokens with readable dark operational contrast

#### Scenario: Use minimal style
- **WHEN** `minimal` style is active
- **THEN** all tab surfaces SHALL use existing semantic theme tokens with crisp low-noise separation

### Requirement: Web session workspace preview
The Web/mock runtime SHALL keep all eight session tabs usable with deterministic data while clearly distinguishing simulated native capabilities.

#### Scenario: Browse Web tab fixtures
- **WHEN** a user opens Files, Changes, Documents, Terminal, Logs, or Report in Web/mock mode
- **THEN** the adapter SHALL return stable mock data suitable for browser preview and automated tests

#### Scenario: Encounter native-only action
- **WHEN** a Web/mock user requests a local-process or local-export operation
- **THEN** the UI SHALL identify the operation as simulated or unavailable and SHALL NOT claim a native side effect

### Requirement: Desktop session-workspace command availability
The desktop runtime SHALL register declared session-workspace and shell commands that implement the frontend session-workspace service contract.

#### Scenario: Invoke a session-workspace operation in desktop mode
- **WHEN** the Tauri session-workspace adapter invokes a declared directory, document, Git, log, or shell operation
- **THEN** the desktop runtime SHALL route the command to its Rust implementation
- **AND** it SHALL return the documented service result rather than an unknown-command error

#### Scenario: Run session workspace in Web mode
- **WHEN** the session workspace runs through the Web/mock adapter
- **THEN** it SHALL retain the existing Web-compatible service behavior without requiring native command registration

### Requirement: Remote workspace local-tab availability
Local filesystem-backed session workspace tabs SHALL avoid reading local paths for remote workspace sessions.

#### Scenario: Open local file-backed tab for remote workspace
- **WHEN** Files, Documents, Changes, or Shell is opened for a remote workspace session before remote execution support exists
- **THEN** the service SHALL return an unavailable workspace context or reject process creation with a concise unsupported message

#### Scenario: Keep chat available for remote workspace
- **WHEN** a remote workspace session is selected
- **THEN** the Chat tab SHALL remain available and the session metadata SHALL identify the remote workspace target

### Requirement: Session folder-opener toolbar action
The session workspace SHALL render an icon-bearing split folder-opener control as a sibling immediately to the right of the eight-tab list, with a main action for the effective default and a menu for enabled available openers.

#### Scenario: Open with the effective default
- **WHEN** an active local session has an authorized directory and an effective default opener
- **THEN** the main action SHALL show that opener's icon and accessible name
- **AND** activating it SHALL request the session folder through the frontend service boundary

#### Scenario: Choose another enabled opener
- **WHEN** the user opens the folder-opener menu
- **THEN** the menu SHALL identify the effective default and list enabled available openers with recognizable icons and localized labels
- **AND** selecting an entry SHALL request that opener without changing the configured default

#### Scenario: Explain configured-default fallback
- **WHEN** the configured default is unavailable and another opener is effective
- **THEN** the control SHALL expose localized fallback feedback rather than silently presenting the replacement as the configured choice

#### Scenario: Preserve tab accessibility
- **WHEN** keyboard or assistive-technology users navigate the session toolbar
- **THEN** only the existing eight tabs SHALL participate in tab-list navigation
- **AND** the opener action and menu SHALL expose their own button and menu keyboard behavior

#### Scenario: Fit a narrow workspace
- **WHEN** the session workspace is too narrow to show all tabs and the opener control
- **THEN** the tab list SHALL retain internal horizontal scrolling
- **AND** the opener control SHALL remain fixed and usable at the right edge

#### Scenario: Disable an unavailable session action
- **WHEN** no session is active, the session has no existing local root, or the session is remote
- **THEN** the opener action SHALL be disabled or expose the corresponding localized unavailable explanation
- **AND** SHALL NOT request a native process launch

### Requirement: Lazy session tab module loading
The session workspace SHALL dynamically load non-default tab modules on first activation and SHALL preserve the existing per-session mounted-tab lifecycle after each module resolves.

#### Scenario: Start a session workspace
- **WHEN** a session becomes active and only the default Chat tab has been visited
- **THEN** non-default tab modules SHALL remain unloaded until activated

#### Scenario: Activate an unloaded tab
- **WHEN** the user activates a non-default tab whose module has not loaded
- **THEN** that panel SHALL show a localized size-stable loading state
- **AND** resolving the module SHALL add the panel to the selected session's mounted-tab set

#### Scenario: Return to a loaded tab
- **WHEN** the user returns to a previously loaded tab in the same session
- **THEN** the panel SHALL retain component state and use CSS visibility for keep-alive behavior

#### Scenario: Switch sessions after loading tabs
- **WHEN** the active session id changes
- **THEN** the previous session's panels SHALL unmount
- **AND** the new session SHALL reset to the eager Chat tab without eagerly mounting non-default panels

#### Scenario: Fail to load a tab module
- **WHEN** a non-default tab module fails to load
- **THEN** only that tab panel SHALL show a localized retry action
- **AND** other mounted tabs SHALL remain operable

