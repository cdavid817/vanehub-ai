## ADDED Requirements

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

