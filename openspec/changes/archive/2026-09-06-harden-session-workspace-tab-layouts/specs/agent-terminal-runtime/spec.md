## ADDED Requirements

### Requirement: Terminal inset painted from the palette
The inset between a terminal's frame and its character grid SHALL be painted in the terminal's own background colour.

#### Scenario: Light CLI theme on WebKitGTK
- **WHEN** the CLI terminal theme is light and the desktop client renders the Agent terminal
- **THEN** the viewport behind the grid SHALL take the light background, so no dark ring appears inside the frame
