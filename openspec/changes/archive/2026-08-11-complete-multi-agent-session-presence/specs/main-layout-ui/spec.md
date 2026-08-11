## ADDED Requirements

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
