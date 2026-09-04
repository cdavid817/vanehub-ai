## ADDED Requirements

### Requirement: Rendered and navigable user guide
The user guide SHALL render authored markup as intended content rather than exposed HTML syntax, and its links SHALL resolve to valid in-application or external destinations.

#### Scenario: Render guide markup
- **WHEN** a user opens a guide page containing supported markup
- **THEN** headings, links, lists, and emphasis render as formatted content

#### Scenario: Follow a guide link
- **WHEN** a user activates a guide link
- **THEN** the target resolves without a 404 destination
