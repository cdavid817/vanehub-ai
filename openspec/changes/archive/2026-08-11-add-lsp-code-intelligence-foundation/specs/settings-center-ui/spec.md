## ADDED Requirements

### Requirement: Settings expose LSP configuration and runtime status
The Agent configuration area SHALL provide a localized service-backed LSP section with the master switch, Rust and TypeScript/JavaScript switches, automatic discovery state, executable override controls, bounded initialization-options validation, trusted-workspace management, isolated server testing, and running-server status. React components SHALL use the shared frontend service boundary, and desktop and Web adapters SHALL implement the same contract shape.

#### Scenario: User configures Rust LSP
- **WHEN** a user enables LSP and Rust, selects a discovered `rust-analyzer` or valid executable override, supplies valid bounded initialization options, and saves
- **THEN** the settings page SHALL submit the normalized configuration through the service boundary
- **AND** it SHALL refresh discovery and affected server status without calling Tauri directly

#### Scenario: Initialization options are invalid
- **WHEN** a user attempts to save malformed, non-object, or oversized initialization-options JSON
- **THEN** shared form validation SHALL reject the submission
- **AND** the last valid persisted configuration SHALL remain active

#### Scenario: User trusts a workspace
- **WHEN** a user grants LSP trust to a canonical local workspace
- **THEN** the UI SHALL explain that a language server is a local executable with the user's operating-system permissions
- **AND** the trusted-workspace list SHALL refresh through the service boundary

#### Scenario: Runtime status is displayed
- **WHEN** one or more language-server instances are starting, ready, backing off, stopping, or failed
- **THEN** the status surface SHALL show safe server identity, language, relative project root, lifecycle state, restart count, last response, and diagnostic count when available
- **AND** it SHALL NOT claim portable memory or indexed-file metrics that the server does not provide

#### Scenario: Web runtime opens LSP settings
- **WHEN** the LSP settings section is used in browser Web mode
- **THEN** it SHALL support deterministic mock configuration, trust, discovery, testing, and status behavior
- **AND** it SHALL not require a native filesystem or process
