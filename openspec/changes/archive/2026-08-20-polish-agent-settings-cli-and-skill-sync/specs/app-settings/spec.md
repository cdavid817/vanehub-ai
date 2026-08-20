## ADDED Requirements

### Requirement: Minimal first-use visual style
The application SHALL use the `minimal` visual style when no valid persisted theme choice is available, while preserving any valid theme explicitly saved by the user.

#### Scenario: Start without a saved theme
- **WHEN** a new installation or cleared Web/mock profile starts without a persisted visual-theme value
- **THEN** the settings service SHALL return `minimal` as the effective theme
- **AND** the formal application surface SHALL first render with the minimal theme applied

#### Scenario: Restore a saved futuristic theme
- **WHEN** the persisted visual-theme value is `futuristic`
- **THEN** the application SHALL restore `futuristic` instead of replacing it with the new default

#### Scenario: Recover from an invalid theme value
- **WHEN** startup encounters a missing, invalid, or unreadable visual-theme value
- **THEN** the runtime SHALL fall back to `minimal` consistently in desktop and Web/mock modes
