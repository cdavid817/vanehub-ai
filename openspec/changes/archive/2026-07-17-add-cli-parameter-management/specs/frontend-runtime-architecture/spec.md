## ADDED Requirements

### Requirement: CLI parameter frontend service contract
The frontend agent service SHALL expose runtime-neutral methods to list, save, and reset typed CLI parameter profiles.

#### Scenario: React loads parameter profiles
- **WHEN** the CLI Parameter Management page needs parameter definitions or selections
- **THEN** the React component SHALL call the frontend agent service interface
- **AND** it SHALL NOT call Tauri `invoke()`, inspect SQLite, or branch on the active runtime

#### Scenario: React mutates parameter profile
- **WHEN** the user saves or resets a CLI profile
- **THEN** the React component SHALL submit typed values through the frontend agent service
- **AND** it SHALL render the normalized profile returned by the service

### Requirement: CLI parameter adapter parity
The Tauri and Web/mock agent adapters SHALL implement the same typed CLI parameter profile contract and validation-visible response shapes.

#### Scenario: Desktop adapter operation
- **WHEN** the frontend runs in the Tauri desktop runtime
- **THEN** only the Tauri-specific adapter SHALL invoke the native list, save, or reset command

#### Scenario: Web adapter operation
- **WHEN** the frontend runs in Web/mock mode
- **THEN** the Web adapter SHALL provide catalog, persistence, reset, validation, and preview behavior without requiring native commands
- **AND** local CLI process launch SHALL remain unavailable in Web/mock mode

### Requirement: CLI parameter contract conformance
Frontend contracts and adapter fixtures SHALL preserve stable agent ids, parameter ids, control kinds, value shapes, and default semantics across runtime implementations.

#### Scenario: Run adapter contract tests
- **WHEN** frontend contract conformance tests execute
- **THEN** both adapters SHALL satisfy the same profile shape and mutation semantics for the four stable CLI agent ids

