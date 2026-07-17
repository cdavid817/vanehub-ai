## ADDED Requirements

### Requirement: Structured archive catalog
The repository SHALL maintain `openspec/changes/archive/archive-index.json` as a generated, deterministic catalog of online archive directories. Each catalog entry SHALL include the archive date, change name, relative archive path, available top-level planning artifacts, and capability names derived from delta-spec directory paths. Archive discovery tools and contributor guidance MUST query this catalog before scanning archived Markdown content.

#### Scenario: Locate archives for a capability
- **WHEN** a maintainer needs the history for a capability
- **THEN** the maintainer can filter `archive-index.json` by that capability without traversing archive directories or searching their Markdown content

#### Scenario: Regenerate the catalog after archiving
- **WHEN** the archive index generator runs after an archive operation
- **THEN** it writes one catalog entry for every date-prefixed online archive directory and refreshes the human-readable README from the same entry set
