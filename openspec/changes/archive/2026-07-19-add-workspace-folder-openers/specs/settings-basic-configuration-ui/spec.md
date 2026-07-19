## ADDED Requirements

### Requirement: Folder-opener settings section
The Basic Configuration page SHALL provide a service-backed folder-opener section for viewing detected programs, choosing one default, selecting enabled openers, and refreshing bounded discovery.

#### Scenario: Display supported opener status
- **WHEN** a user opens Basic Configuration
- **THEN** the page SHALL list all supported opener ids with localized name, recognizable icon, availability state, and resolved version, edition, or executable path when provided

#### Scenario: Configure enabled openers
- **WHEN** a user changes the multi-select opener list
- **THEN** the page SHALL keep File Explorer selected as the required fallback
- **AND** SHALL save the complete preference aggregate through the service boundary

#### Scenario: Configure the default opener
- **WHEN** a user selects an enabled available opener as default
- **THEN** the page SHALL atomically save it with the enabled list
- **AND** the session toolbar SHALL observe the coherent preference change

#### Scenario: Prevent an unavailable default
- **WHEN** an opener is not installed, invalid, unsupported, or failed detection
- **THEN** the page SHALL display its status
- **AND** SHALL prevent selecting it as a new default while retaining any existing enabled selection

#### Scenario: Refresh local discovery
- **WHEN** the user activates the refresh action
- **THEN** the page SHALL show a non-blocking detection state and request a fresh bounded scan through the service boundary
- **AND** SHALL update per-opener results without changing saved preference selections

#### Scenario: Render Web preview limitations
- **WHEN** the settings section runs through the Web/mock adapter
- **THEN** it SHALL remain interactive with deterministic data
- **AND** SHALL identify native installation status and launch behavior as simulated or unavailable

