## ADDED Requirements

### Requirement: Context quality service adapter parity
The frontend service boundary SHALL expose typed context-quality history and summary operations implemented by both the Tauri and Web/mock adapters, and React components SHALL NOT access native commands, SQLite, or diagnostic logs directly.

#### Scenario: Desktop context health request
- **WHEN** the frontend requests OnePiece context-quality history in the desktop runtime
- **THEN** the Tauri adapter SHALL map the typed request and response through native commands

#### Scenario: Browser context health request
- **WHEN** the same frontend surface runs in Web/mock mode
- **THEN** the Web adapter SHALL return the same contract shape using deterministic bounded mock records without a network request

#### Scenario: Runtime reports a typed failure
- **WHEN** either adapter cannot load context-quality data
- **THEN** the service boundary SHALL preserve a typed safe error that the UI can present without runtime-specific branching

