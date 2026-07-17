## ADDED Requirements

### Requirement: Localized workspace shell text
The workspace shell SHALL render sidebar, status bar, information panel, session actions, and create-session dialog text through synchronized zh-CN and en translation resources.

#### Scenario: Create-session dialog localized
- **WHEN** the create-session dialog renders in Simplified Chinese or English
- **THEN** its title, description, project folder labels, browse action, Git/worktree helper text, worktree labels, session name labels, placeholders, create action, cancel action, and user-facing validation errors SHALL use the active locale

#### Scenario: Workspace panel labels localized
- **WHEN** the workspace shell renders sidebar, main content, information panel, status bar, or context menus in Simplified Chinese or English
- **THEN** user-visible labels, tab names, badges, context actions, confirmations, empty states, and helper text SHALL use the active locale

#### Scenario: Workspace date formatting localized
- **WHEN** workspace session cards or message-adjacent UI render user-visible dates
- **THEN** date formatting SHALL follow the active application language rather than always using a fixed locale

#### Scenario: Preserve workspace identifiers
- **WHEN** the workspace shell displays Agent ids, interaction mode ids, project paths, worktree names, branch names, or command-like values
- **THEN** those values MAY remain literal while surrounding labels and helper text use the active locale
