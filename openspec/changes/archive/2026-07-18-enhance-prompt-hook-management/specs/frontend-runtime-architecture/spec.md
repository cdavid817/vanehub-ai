## ADDED Requirements

### Requirement: Prompt Hook frontend service boundary
The frontend Agent service SHALL expose runtime-neutral Prompt Hook management, preview, and trace query methods.

#### Scenario: React manages Prompt Hooks
- **WHEN** a React settings component lists, creates, edits, deletes, enables, disables, binds, previews, or inspects Prompt Hooks
- **THEN** it SHALL call the frontend Agent service interface
- **AND** it SHALL NOT call Tauri `invoke()`, inspect SQLite, or import native-only Prompt Hook code

#### Scenario: Tauri Prompt Hook adapter
- **WHEN** the frontend runs inside the Tauri desktop runtime and Prompt Hook data is requested or mutated
- **THEN** only the Tauri-specific Agent service adapter SHALL invoke declared native Prompt Hook commands

#### Scenario: Web Prompt Hook adapter
- **WHEN** the frontend runs outside the Tauri desktop runtime
- **THEN** the Web/mock Agent service adapter SHALL provide the same Prompt Hook method signatures and compatible normalized result shapes without requiring native commands

### Requirement: Prompt Hook adapter contract parity
The Tauri and Web/mock adapters SHALL preserve Prompt Hook model shapes and mutation semantics.

#### Scenario: Contract conformance test
- **WHEN** frontend adapter contract tests run
- **THEN** both runtime adapters SHALL support the same Prompt Hook hook shape, category values, source values, CLI binding values, trace summary shape, preview shape, and mutation result semantics
