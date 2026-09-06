## ADDED Requirements

### Requirement: Collapsible terminal composer
The single-Agent CLI terminal tab SHALL let the user fold the line composer beneath the terminal away and bring it back, because the terminal itself accepts input directly.

#### Scenario: Collapse the composer
- **WHEN** the user activates the collapse control
- **THEN** the composer SHALL shrink to one bar carrying an expand control and a localized hint that input can be typed directly in the terminal
- **AND** the terminal SHALL reclaim the freed height and refit its grid

#### Scenario: Expand the composer
- **WHEN** the user activates the expand control
- **THEN** the full composer SHALL return with any unsent draft intact

#### Scenario: Remember the choice per viewer
- **WHEN** the composer was collapsed and a terminal tab is shown again later in the same browser profile
- **THEN** it SHALL start collapsed
- **AND** an unavailable storage SHALL fall back to expanded without error

#### Scenario: Keep the control accessible
- **WHEN** either control renders
- **THEN** it SHALL carry a localized accessible name and be keyboard operable
