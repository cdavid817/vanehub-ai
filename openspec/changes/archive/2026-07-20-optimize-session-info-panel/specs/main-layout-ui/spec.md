## ADDED Requirements

### Requirement: Optimized information panel tabs
The information panel SHALL provide keep-alive tabs for Basic Info, Token Usage, and Skill.

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

#### Scenario: Show relevant Skills
- **WHEN** the Skill tab is visible for an active session
- **THEN** the tab SHALL show available Skills for the selected CLI separately from project Skills discovered for the active workspace
- **AND** disabled project Skills SHALL be visually de-emphasized and SHALL NOT be included in the available Skills group

#### Scenario: Localize optimized information panel
- **WHEN** the optimized information panel renders in Simplified Chinese or English
- **THEN** all user-visible labels, tab names, loading states, empty states, and section headings SHALL use the active locale resources
- **AND** stable Agent ids, model ids, project paths, worktree names, and Skill ids MAY remain literal identifiers

#### Scenario: Preserve compact panel behavior
- **WHEN** the optimized information panel renders in `futuristic` or `minimal` style
- **THEN** it SHALL use shared semantic panel, muted-panel, segmented-control, border, text, and status tokens
- **AND** long labels, model ids, paths, and Skill names SHALL not overlap adjacent controls or resize the workspace grid

## REMOVED Requirements

### Requirement: Information panel tabs
**Reason**: The compact information panel now focuses on active CLI session metadata, session-scoped usage, and relevant Skills. Files, Changes, Logs, hard-coded task progress, and compact terminal diagnostics are covered by richer workspace surfaces or replaced by the optimized tabs.

**Migration**: Use the new Optimized information panel tabs requirement for Basic Info, Token Usage, and Skill behavior.
