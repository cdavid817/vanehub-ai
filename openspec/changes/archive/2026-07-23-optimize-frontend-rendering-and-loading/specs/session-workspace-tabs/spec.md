## ADDED Requirements

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
