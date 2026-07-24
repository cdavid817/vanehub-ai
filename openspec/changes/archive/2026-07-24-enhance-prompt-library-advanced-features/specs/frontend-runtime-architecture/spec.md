## ADDED Requirements

### Requirement: Prompt Hook advanced-feature adapter contract
The frontend Agent service SHALL expose runtime-neutral variable-catalog, draft, publication, version-history, rollback, and version-evaluation methods with equivalent Tauri and Web/mock signatures.

#### Scenario: React manages Prompt Hook versions
- **WHEN** a React settings component discovers variables, saves a draft, publishes, lists versions, rolls back, or loads evaluation summaries
- **THEN** it SHALL call only the frontend Agent service interface
- **AND** it SHALL NOT invoke Tauri commands, inspect SQLite, or calculate native execution metrics directly

#### Scenario: Tauri adapter invokes bounded commands
- **WHEN** the desktop frontend requests a Prompt Hook advanced operation
- **THEN** only the Tauri-specific adapter SHALL invoke its declared native command and normalize the response

#### Scenario: Web adapter preserves behavior
- **WHEN** the Web/mock runtime performs the same operation
- **THEN** it SHALL preserve compatible draft, immutable-version, rollback, variable-rendering, and evaluation-summary semantics without accessing native SQLite or launching a CLI
