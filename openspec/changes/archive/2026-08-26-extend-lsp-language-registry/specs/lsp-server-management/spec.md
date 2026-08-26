## RENAMED Requirements

- FROM: `### Requirement: Rust and TypeScript servers are discoverable and testable`
- TO: `### Requirement: Registered language servers are discoverable and testable`

## ADDED Requirements

### Requirement: Supported languages are defined by a language registry
The system SHALL define every supported language as a registry entry carrying a stable language id, candidate server executable names in preference order, project-root markers, source-file extension to LSP language identifier mapping, default startup arguments, default initialization options, and platform applicability. Configuration, discovery, project-root detection, document admission, server testing, and the settings surface SHALL derive their supported-language set from that registry. Persisted configuration SHALL accept any registered language id, and the storage layer SHALL NOT constrain which language ids may exist.

#### Scenario: Registry is the single source of supported languages
- **WHEN** the registry declares a language
- **THEN** configuration persistence, discovery, project-root detection, document admission, server testing, and the settings surface SHALL all treat that language as supported
- **AND** no additional storage-layer change SHALL be required to persist its configuration

#### Scenario: Persisted configuration names an unregistered language
- **WHEN** stored configuration contains a language id that the running build does not register
- **THEN** the system SHALL ignore that entry, report the effective configuration for registered languages only, and preserve the unknown row unchanged
- **AND** it SHALL NOT fail startup, reject the whole configuration, or start a server for the unknown id

#### Scenario: Request names an unregistered language
- **WHEN** a command or tool request supplies a language id that is not registered
- **THEN** the system SHALL reject the request with a safe reason code
- **AND** it SHALL NOT start a process or fall back to another language

#### Scenario: Language declares no support for the host platform
- **WHEN** a registered language declares no applicability for the current operating system
- **THEN** discovery SHALL report it as unavailable with a platform reason
- **AND** the settings surface SHALL present it as unsupported on this host rather than as merely undiscovered

## MODIFIED Requirements

### Requirement: LSP activation is explicitly configured and trusted
The system SHALL persist an LSP master switch, one independent switch per registered language, bounded server initialization options, bounded startup arguments, optional executable overrides, and trust for canonical local workspaces. All switches and workspace trust SHALL default to disabled, and code-index enablement SHALL NOT imply LSP trust.

#### Scenario: Fresh installation does not start a server
- **WHEN** the application starts without saved LSP configuration
- **THEN** the LSP master switch and every language switch SHALL be disabled
- **AND** no language-server process SHALL start

#### Scenario: Indexed workspace is not trusted for LSP
- **WHEN** a workspace has local or semantic code indexing enabled but has not been trusted for LSP
- **THEN** the system SHALL NOT start a language server for that workspace

#### Scenario: Trust is revoked
- **WHEN** a user revokes LSP trust for a canonical workspace
- **THEN** the system SHALL reject new requests for that workspace
- **AND** it SHALL gracefully stop every language-server process owned by that workspace

#### Scenario: Newly registered language defaults to disabled
- **WHEN** a build registers a language for which no configuration has been saved
- **THEN** that language's switch SHALL read as disabled
- **AND** no server SHALL start for it until a user enables it and trusts a workspace

#### Scenario: Startup arguments are invalid
- **WHEN** a user saves startup arguments that are not a bounded list of strings, or that exceed the declared size limit
- **THEN** the system SHALL reject the save with a safe reason code
- **AND** the last valid persisted configuration SHALL remain active

#### Scenario: Startup arguments are omitted
- **WHEN** a language has no user-supplied startup arguments
- **THEN** the system SHALL start its server with the registry-declared default arguments for that language

### Requirement: Registered language servers are discoverable and testable
The desktop runtime SHALL discover each enabled registered language's declared server executables, in the registry's preference order, from a configured absolute override or the native executable search path, and SHALL test a discovered server through an isolated bounded initialize and shutdown lifecycle without opening an interactive session. Each registered language SHALL declare the startup arguments its server requires, and every currently registered language SHALL communicate over stdio.

#### Scenario: Executable is automatically discovered
- **WHEN** an enabled server has no manual executable override and a supported executable exists on the native search path
- **THEN** discovery SHALL report the resolved server kind and an available status
- **AND** discovery SHALL NOT start a persistent workspace server

#### Scenario: Manual override is unavailable
- **WHEN** a configured executable override does not resolve to an executable file
- **THEN** discovery and server testing SHALL report an unavailable result with a safe reason code
- **AND** no fallback executable SHALL be started silently

#### Scenario: Server test completes
- **WHEN** a user tests an available server configuration
- **THEN** the desktop runtime SHALL use an isolated minimal local project, complete `initialize` and `initialized`, send `shutdown` and `exit`, and return bounded phase results
- **AND** the test SHALL clean up the child process within a fixed deadline

#### Scenario: Language declares several candidate executables
- **WHEN** a registered language declares more than one candidate executable name and several of them resolve on the native search path
- **THEN** discovery SHALL select the first candidate in the registry's declared preference order
- **AND** it SHALL report which candidate was selected

#### Scenario: Server test uses the language's own fixture project
- **WHEN** a user tests a registered language's server
- **THEN** the isolated minimal project SHALL be the one the registry declares for that language
- **AND** the test SHALL NOT reuse another language's project layout
