## ADDED Requirements

### Requirement: Protected default branch
The GitHub repository SHALL protect the default branch from deletion, force pushes, and unvalidated direct changes, and SHALL require pull requests with resolved review conversations and successful required checks.

#### Scenario: Pull request targets main
- **WHEN** a contributor proposes a change to `main`
- **THEN** GitHub SHALL require the configured CI checks and all review conversations to resolve before merge

#### Scenario: Destructive branch update
- **WHEN** an actor attempts to delete `main` or update it with a non-fast-forward push
- **THEN** the repository ruleset SHALL reject the operation

### Requirement: Public contribution guidance
The repository SHALL publish ownership, contribution, conduct, support, vulnerability-reporting, issue, and pull-request guidance consistent with the project's OpenSpec and runtime-boundary rules.

#### Scenario: Contributor opens a pull request
- **WHEN** a contributor prepares a pull request
- **THEN** the template SHALL prompt for OpenSpec impact, adapter parity, logging constraints, required validation, and UI evidence where applicable

#### Scenario: Reporter discovers a vulnerability
- **WHEN** a reporter reads the repository security policy
- **THEN** the policy SHALL direct confidential reports to GitHub private vulnerability reporting instead of a public issue

### Requirement: Consistent merge and ownership settings
The repository SHALL use a documented ownership map, squash-oriented merge behavior, automatic feature-branch deletion, and labels that identify major project areas.

#### Scenario: Pull request is merged
- **WHEN** a feature pull request is merged
- **THEN** GitHub SHALL allow the configured squash workflow and delete the merged head branch automatically

#### Scenario: Repository area changes
- **WHEN** a pull request modifies paths covered by the label configuration
- **THEN** repository automation SHALL apply the corresponding project-area labels
