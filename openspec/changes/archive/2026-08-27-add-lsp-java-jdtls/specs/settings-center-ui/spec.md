## MODIFIED Requirements

### Requirement: Settings expose LSP configuration and runtime status
The Agent configuration area SHALL provide a localized service-backed LSP section with the master switch, one switch per registered language obtained from the service boundary, automatic discovery state, override controls whose meaning follows each language's backend-reported launch shape, bounded startup-argument controls, bounded initialization-options validation, trusted-workspace management, isolated server testing, and running-server status. The section SHALL render its language controls from the backend-supplied registered-language set, and its negotiated-capability rows from the backend-supplied negotiated method list, rather than from fixed lists compiled into the frontend. React components SHALL use the shared frontend service boundary, and desktop and Web adapters SHALL implement the same contract shape.

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

#### Scenario: Registered language set determines the rendered controls
- **WHEN** the settings section loads the registered-language set through the service boundary
- **THEN** it SHALL render exactly one language control group per registered language, each with that language's own discovery state, override, startup arguments, and initialization options
- **AND** adding a language to the backend registry SHALL require no new per-language frontend component

#### Scenario: A language's override names a directory rather than a file
- **WHEN** the backend reports a language whose launch shape takes an install directory
- **THEN** the override control SHALL describe and validate a directory rather than an executable file
- **AND** it SHALL do so from the reported launch shape, not from the language's identity, so a second such language needs no frontend change

#### Scenario: A prerequisite runtime is missing
- **WHEN** discovery reports that a language's prerequisite runtime is absent
- **THEN** the language card SHALL present that as its own state, distinct from an unset install directory and from a directory missing its launcher
- **AND** it SHALL name the runtime the user has to install rather than reporting a generic unavailable server

#### Scenario: Language is unsupported on this host
- **WHEN** a registered language declares no applicability for the current operating system
- **THEN** its control group SHALL present it as unsupported on this host and SHALL NOT offer enablement or server testing
- **AND** it SHALL be distinguishable from a supported language whose executable was simply not discovered

#### Scenario: Startup arguments are invalid
- **WHEN** a user attempts to save startup arguments that are not a bounded list of strings or that exceed the declared size limit
- **THEN** shared form validation SHALL reject the submission
- **AND** the last valid persisted configuration SHALL remain active

#### Scenario: Negotiated method list determines the rendered capability rows
- **WHEN** the status surface renders a ready server's negotiated capabilities
- **THEN** it SHALL render one supported-or-unsupported row per method the backend reports, in the order reported
- **AND** adding a method to the backend SHALL require no new frontend row

#### Scenario: A reported method has no localized label
- **WHEN** the backend reports a negotiated method whose localization key is absent from the active locale
- **THEN** the row SHALL fall back to the raw method identifier
- **AND** it SHALL NOT render the missing key or an empty label
