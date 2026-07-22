## ADDED Requirements

### Requirement: SSH connection settings page
The settings center SHALL provide a service-backed SSH connection management page.

#### Scenario: Open SSH connection settings
- **WHEN** a user selects the SSH connection settings entry
- **THEN** the page SHALL load SSH connection profiles through a frontend service interface
- **AND** React components SHALL NOT call Tauri commands directly

#### Scenario: Empty SSH connection state
- **WHEN** no SSH connections are configured
- **THEN** the page SHALL show an empty state with an action to add the first connection

#### Scenario: Search SSH connections
- **WHEN** a user enters a search term on the SSH connection settings page
- **THEN** the page SHALL filter visible profiles by name, host, user, default path, and test state

### Requirement: SSH connection form behavior
The SSH connection settings page SHALL provide localized form controls for profile metadata and write-only credentials.

#### Scenario: Add password connection
- **WHEN** a user adds a password-authenticated SSH connection
- **THEN** the form SHALL collect name, host, port, user, default path, and password
- **AND** the submitted password SHALL be treated as write-only input

#### Scenario: Edit stored password connection
- **WHEN** a user edits an SSH connection that already has a stored password
- **THEN** the form SHALL show a localized configured indicator or redacted placeholder
- **AND** the form SHALL preserve the existing password unless the user enters a replacement

#### Scenario: Add key connection
- **WHEN** a user adds a key-authenticated SSH connection
- **THEN** the form SHALL collect name, host, port, user, default path, and key path
- **AND** the form SHALL NOT request or store private key contents

### Requirement: SSH connection settings actions
The SSH connection settings page SHALL support high-frequency management actions through the service boundary.

#### Scenario: Test SSH connection from settings
- **WHEN** a user activates the test action for an SSH connection
- **THEN** the page SHALL start the service-backed test operation and show pending, succeeded, or failed feedback without exposing secrets

#### Scenario: Delete SSH connection from settings
- **WHEN** a user confirms deletion of an SSH connection
- **THEN** the page SHALL request deletion through the service boundary
- **AND** it SHALL refresh visible profiles after the mutation completes

### Requirement: Localized SSH connection UI
The SSH connection settings page and dialogs SHALL render user-visible text through synchronized zh-CN and en translation resources.

#### Scenario: Render localized SSH settings
- **WHEN** the SSH connection settings page renders in Simplified Chinese or English
- **THEN** page titles, descriptions, actions, labels, placeholders, validation messages, status text, confirmations, notices, and empty states SHALL use the active locale
