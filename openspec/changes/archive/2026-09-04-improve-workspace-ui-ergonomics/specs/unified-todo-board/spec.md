## MODIFIED Requirements

### Requirement: Accessible responsive board interaction
The board SHALL support pointer, keyboard, and compact-layout operation without requiring drag and drop, and SHALL present work items with enough visual weight separation to be scanned rather than read in full.

#### Scenario: Move without dragging
- **WHEN** a keyboard or assistive-technology user changes a card stage or position
- **THEN** the board SHALL provide explicit controls that perform the same persisted mutation as pointer movement

#### Scenario: Compact viewport
- **WHEN** the available width cannot show all stages at once
- **THEN** the board SHALL preserve access to every stage, filter, card action, and source status without clipping required controls

#### Scenario: Scan a work item card
- **WHEN** a work item card renders on the board
- **THEN** its title SHALL be the most prominent element and its priority, stage, project, sources, and due date SHALL be visually subordinate to it
- **AND** card actions SHALL be grouped separately from card content

#### Scenario: Distinguish stage columns
- **WHEN** the board renders its stage columns
- **THEN** each column SHALL identify its stage and its matching item count
- **AND** a column that is empty SHALL state whether it is empty because of active filters or because it has no items

#### Scenario: Group board filters
- **WHEN** the board header renders its discovery controls
- **THEN** search SHALL be distinguishable from the categorical filters
- **AND** the header SHALL indicate when filters are narrowing the board

## ADDED Requirements

### Requirement: User-safe board path display
Every board surface that shows a work item's project path SHALL apply the application's user-safe path display rule.

#### Scenario: Card shows an extended-length path
- **WHEN** a work item's stored project path is `\\?\D:\cdavid\Documents\code\cc-switch`
- **THEN** the card SHALL display `D:\cdavid\Documents\code\cc-switch`
- **AND** its hover title SHALL show the same normalized path

#### Scenario: Project filter shows extended-length paths
- **WHEN** the board builds its project filter options from stored project paths
- **THEN** each option SHALL display the normalized path
- **AND** selecting an option SHALL still filter against the stored path value
