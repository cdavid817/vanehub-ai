## ADDED Requirements

### Requirement: Non-duplicative CLI installation summary
The CLI Management page SHALL summarize installation coverage with one installed-CLI metric and SHALL NOT render a second missing-CLI metric that repeats the inverse value.

#### Scenario: Render CLI installation summary
- **WHEN** CLI statuses have loaded
- **THEN** the page SHALL display the installed count against the total managed CLI count
- **AND** it SHALL NOT display a separate uninstalled-count summary card

