## ADDED Requirements

### Requirement: User-facing task workspace paths
The task board SHALL render a task workspace path without an operating-system namespace prefix that is not meaningful to the user.

#### Scenario: Windows extended path is displayed
- **WHEN** a task workspace path begins with a Windows extended-length namespace prefix
- **THEN** the board displays its ordinary user-facing path
- **AND** copying or opening the workspace uses the original valid path value
