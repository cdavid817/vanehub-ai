## ADDED Requirements

### Requirement: Responsive selected settings navigation
The settings navigation SHALL render the active submenu indicator without clipping its left or right border at supported widths.

#### Scenario: Resize with a selected submenu
- **WHEN** a user resizes the settings window while a submenu is selected
- **THEN** the selected state remains fully visible and distinguishable

### Requirement: Current About presentation
The About page SHALL present stable release information without a preview label for a stable build.

#### Scenario: Stable build opens About
- **WHEN** the installed application version has no semantic-version prerelease identifier
- **THEN** the About page does not render a preview label
