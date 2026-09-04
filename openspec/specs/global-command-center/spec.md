# global-command-center Specification

## Purpose
Lets users jump directly to a specific Goal from the Global Command Center (`Ctrl/Cmd+K`) by
searching its title, without first navigating to the Plan destination and its Goals tab.
## Requirements
### Requirement: Goal search in the Global Command Center
The Global Command Center SHALL provide a Goal search scope that matches goals by title and
navigates a selected result to that goal's location in the Plan destination's Goals tab.

#### Scenario: Matching a goal by title
- **WHEN** a user opens the Command Center and types a query that matches an existing goal's
  title (case-insensitive substring)
- **THEN** the matching goal appears in the results list with its title and a status indicator
  derived from its current status

#### Scenario: Selecting a goal result navigates to it
- **WHEN** a user selects a goal result from the Command Center
- **THEN** the app navigates to the Plan destination's Goals tab with that goal selected, and the
  Command Center closes

#### Scenario: No results for a non-matching query
- **WHEN** a user types a query that matches no goal's title
- **THEN** the Goal scope contributes no results, and no error is shown

#### Scenario: Result content excludes sensitive fields
- **WHEN** a goal search result is built
- **THEN** it includes only the goal's id, title, status, and last-updated time -- never its
  description, acceptance notes, or linked-target identifiers

