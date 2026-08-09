## ADDED Requirements

### Requirement: Antigravity CLI parameter catalog
The backend-authoritative catalog SHALL define `antigravity-cli` parameters for model selection (`--model`, composite custom-text control), reasoning effort (`--effort`, enum over `low`, `medium`, `high`), execution mode (`--mode`, enum over `plan` and `accept-edits`), agent selection (`--agent`, composite custom-text control), and terminal sandbox (`--sandbox`, boolean). The catalog SHALL NOT expose `-p`, `--output-format`, or `--conversation`, which the provider invocation builder owns, and SHALL NOT expose `--dangerously-skip-permissions`, which the existing prohibition on bypass flags excludes.

#### Scenario: Load the Antigravity parameter catalog
- **WHEN** the `antigravity-cli` parameter catalog is loaded
- **THEN** it SHALL contain entries for `--model`, `--effort`, `--mode`, `--agent`, and `--sandbox`

#### Scenario: Managed invocation arguments are absent from the catalog
- **WHEN** the `antigravity-cli` parameter catalog is loaded
- **THEN** it SHALL NOT contain an entry whose literal flag is `-p`, `--output-format`, or `--conversation`

#### Scenario: The permission bypass flag is absent from the catalog
- **WHEN** the `antigravity-cli` parameter catalog is loaded
- **THEN** it SHALL NOT contain an entry whose flag contains `dangerously`
- **AND** a permissive tool-approval posture SHALL be reachable only through the agent's CLI configuration profile, not through a launch parameter

#### Scenario: Preview reflects saved selections
- **WHEN** a user saves `antigravity-cli` selections including a non-default `--effort` value
- **THEN** the returned safe argument preview SHALL include `--effort` with that value

## MODIFIED Requirements

### Requirement: Managed CLI parameter profiles
The system SHALL provide one typed launch-parameter profile for each managed CLI stable agent id: `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, and `antigravity-cli`.

#### Scenario: List managed profiles
- **WHEN** the CLI Parameter Management page loads
- **THEN** the system SHALL return profiles for the five managed stable agent ids in their configured display order
- **AND** each profile SHALL contain definitions, effective selections, defaults, and a safe argument preview

#### Scenario: Reject unknown agent profile
- **WHEN** a client requests or saves a parameter profile for an unknown agent id
- **THEN** the service SHALL reject the request without persisting any selection
