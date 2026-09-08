## MODIFIED Requirements

### Requirement: Routed frontend surfaces
The frontend SHALL expose top-level application surfaces through a routing layer that can address workspace, settings, and future detail views without relying on a single component-local view flag. Workspace destinations, their sub-views, and the active session SHALL be addressable through that routing layer.

#### Scenario: Navigate to settings
- **WHEN** a user opens the settings route in the Tauri desktop runtime or browser Web runtime
- **THEN** the frontend SHALL render the settings center through the route while preserving runtime adapter boundaries

#### Scenario: Navigate to workspace
- **WHEN** a user opens the workspace route in the Tauri desktop runtime or browser Web runtime
- **THEN** the frontend SHALL render the workspace surface through the route without requiring a Tauri-only backend call

#### Scenario: Address a workspace destination
- **WHEN** a user activates the Sessions, Inbox, or Automations destination, or one of their sub-views
- **THEN** the active destination and sub-view SHALL be derived from the route rather than from component-local state
- **AND** opening that route directly SHALL render the same destination and sub-view
- **AND** a retired destination path SHALL resolve to its redirect target without rendering an empty region

#### Scenario: Address the active session
- **WHEN** a session is selected
- **THEN** the route SHALL identify it
- **AND** opening that route directly SHALL select the same session, or report that it is unavailable when it no longer exists

#### Scenario: Reverse navigation
- **WHEN** the user triggers browser or keyboard Back after changing destination or session
- **THEN** the workspace SHALL return to the previous destination or session

#### Scenario: Restore on launch
- **WHEN** the application starts after a previous session ended inside the workspace
- **THEN** it SHALL restore the previous workspace location
- **AND** a remembered retired destination SHALL be redirected the same way a typed path is
