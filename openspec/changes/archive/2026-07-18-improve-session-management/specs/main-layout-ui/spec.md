## ADDED Requirements

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
