## ADDED Requirements

### Requirement: Workspace bottom boundary
The workspace shell SHALL render a continuous one-pixel, theme-aware divider at its bottom edge so the application content remains visually distinct from the operating-system taskbar or adjacent desktop surface.

#### Scenario: Render workspace above the operating-system taskbar
- **WHEN** the main workspace is visible in desktop or Web mode
- **THEN** a non-interactive divider SHALL span the complete workspace width at the bottom edge
- **AND** the divider SHALL use the active theme's border semantics without reducing usable content height

