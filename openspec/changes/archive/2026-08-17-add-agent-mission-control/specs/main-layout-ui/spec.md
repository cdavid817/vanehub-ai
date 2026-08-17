## ADDED Requirements

### Requirement: Mission Control workspace destination
The workspace activity bar SHALL expose a localized icon-only Mission Control destination in both Tauri and Web runtimes, preserve mounted Session workspace state while it is active, and remain operable at desktop and narrow widths.

#### Scenario: Open Mission Control
- **WHEN** the user activates the Mission Control activity entry
- **THEN** the workspace opens the Mission Control surface without a runtime-specific component branch
- **AND** preserves the selected Session and its mounted tab state for return

#### Scenario: Identify Mission Control entry
- **WHEN** the Mission Control icon-only entry is available
- **THEN** it provides localized accessible name, tooltip, focus, hover, and active states without shifting adjacent entries
