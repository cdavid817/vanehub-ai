## ADDED Requirements

### Requirement: Expanded catalog filtering
The Skills settings experience SHALL support filters for first-party origin, Role or Utility type, delivery, category, dependency availability, assigned state, and override state in combination with existing search and Agent selection.

#### Scenario: Filter first-party Utilities
- **WHEN** a user selects first-party origin and Utility type
- **THEN** the inventory SHALL show only effective first-party Utility identities and SHALL preserve any active category, availability, search, and Agent filters

#### Scenario: Filter unavailable dependencies
- **WHEN** a user selects dependency-unavailable status
- **THEN** the inventory SHALL show packages whose advertised behavior is blocked and their safe actionable reasons

#### Scenario: Clear expanded filters
- **WHEN** a user clears expanded catalog filters
- **THEN** the page SHALL restore default filter values without changing the selected stable Agent or management context

### Requirement: First-party catalog statistics
The Skills settings summary SHALL present bounded counts for the 28 first-party packages, effective Role and Utility totals, category distribution, available and dependency-blocked packages, assigned and unassigned packages, and higher-layer overrides.

#### Scenario: Default catalog summary
- **WHEN** the expanded catalog is healthy with no overrides
- **THEN** the summary SHALL show 28 first-party packages, 13 Roles, and 15 Utilities together with availability and assignment counts

#### Scenario: Override does not inflate active total
- **WHEN** a higher-layer Skill shadows a first-party canonical id
- **THEN** the UI SHALL keep one effective Skill row and show the first-party base as overridden rather than increasing the effective total

### Requirement: First-party package presentation
First-party Skill rows and details SHALL display localized name and description, canonical id, category, type, delivery, version, aliases, dependency and modality status, resource summary, immutable base origin, and effective override state using bounded layouts.

#### Scenario: Code review alias displayed
- **WHEN** `code-review` details are opened
- **THEN** the UI SHALL display `code-reviewer` as an alias while using `code-review` for all service operations

#### Scenario: Utility waiting for delegation
- **WHEN** a first-party Utility is unavailable because delegation support is absent
- **THEN** its row SHALL explain the missing runtime dependency and SHALL NOT offer Role injection or misleading execution controls

#### Scenario: External integration not configured
- **WHEN** an integration-dependent package is selected without configuration
- **THEN** details SHALL show setup guidance and SHALL NOT display credential values or trigger account access

#### Scenario: Responsive resource summary
- **WHEN** a package contains references, templates, assets, or inert scripts
- **THEN** the row SHALL remain compact and the detailed resource counts and paths SHALL appear in the existing details presentation without horizontal page scrolling

### Requirement: Catalog UI adapter parity
Desktop and Web/mock Skills settings SHALL consume the same frontend service contracts for first-party categories, classifications, dependencies, aliases, resources, summaries, and unavailable states.

#### Scenario: Web expanded catalog
- **WHEN** the Web/mock adapter returns representative Role, Utility, overridden, assigned, and dependency-unavailable first-party packages
- **THEN** the UI SHALL render the same filtering, statistics, identity, and availability semantics as equivalent desktop data

