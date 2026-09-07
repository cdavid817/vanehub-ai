## ADDED Requirements

### Requirement: Board columns stay reachable at every window size
The board SHALL keep every stage column and every card reachable without clipping.

#### Scenario: Full column
- **WHEN** a stage holds more cards than its column can show
- **THEN** the column's list SHALL scroll inside the board
- **AND** the column SHALL end inside the board rather than under its clip

#### Scenario: Narrow board
- **WHEN** the board is narrower than five columns at their minimum width
- **THEN** the column strip SHALL scroll horizontally with an always-visible track, and each column SHALL be no narrower than a card's action row needs
