## MODIFIED Requirements

### Requirement: Workspace activity bar
The workspace shell SHALL render a persistent icon-only activity bar at the far left of the workspace body in both the Tauri desktop frontend and browser Web runtime.

#### Scenario: Render activity entries
- **WHEN** the workspace activity bar renders
- **THEN** it SHALL show Session, Loops, and Scheduled Tasks entries in a top group
- **AND** it SHALL show Settings and Help entries anchored in a bottom group
- **AND** the entries SHALL display icons without visible text labels

#### Scenario: Identify icon-only entries
- **WHEN** an activity-bar entry is available to pointer, keyboard, or assistive-technology users
- **THEN** it SHALL provide a synchronized zh-CN and en accessible name and tooltip
- **AND** it SHALL expose stable hover, focus, and active styling without shifting adjacent controls

#### Scenario: Open settings from activity bar
- **WHEN** the user activates the Settings activity entry
- **THEN** the system SHALL open the existing settings center without requiring a runtime-specific backend call

#### Scenario: Open Loops from activity bar
- **WHEN** the user activates the Loops activity entry
- **THEN** the workspace SHALL open the Loop Center as the active workspace destination
- **AND** it SHALL preserve mounted session workspace state for later return

#### Scenario: Return to sessions from activity bar
- **WHEN** the user activates the Session activity entry while the Loop Center is active
- **THEN** the workspace SHALL restore the session workspace without losing its selected session and mounted tab state

#### Scenario: Open scheduled tasks from activity bar
- **WHEN** the user activates the Scheduled Tasks activity entry
- **THEN** the workspace SHALL open the scheduled-task management dialog
- **AND** it SHALL NOT show a coming-soon placeholder

#### Scenario: Preserve future help entry
- **WHEN** the activity bar renders its bottom group
- **THEN** it SHALL keep the Help entry available without introducing a new Help destination in this change

