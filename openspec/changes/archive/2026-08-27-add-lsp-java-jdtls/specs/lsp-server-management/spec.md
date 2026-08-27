## ADDED Requirements

### Requirement: A language may declare an interpreter launch shape

A registered language MAY declare that its server runs through a host interpreter with a resolved argument template rather than as a directly executable file. The system SHALL resolve that template before the process starts, SHALL fail closed with a distinct reason for each part of the template it cannot resolve, and SHALL NOT start the interpreter with an unresolved placeholder.

#### Scenario: An interpreter-shaped server starts

- **WHEN** an enabled interpreter-shaped language has its interpreter available and its install directory resolved
- **THEN** the process SHALL start as the interpreter with the resolved argument template
- **AND** the server SHALL communicate over stdio exactly as an executable-shaped server does

#### Scenario: A template placeholder cannot be resolved

- **WHEN** any placeholder in the argument template cannot be resolved
- **THEN** the launch SHALL be refused with a reason naming which part failed
- **AND** the interpreter SHALL NOT be started with the placeholder left in place, and no other launch shape SHALL be substituted

#### Scenario: A user supplies startup arguments for an interpreter-shaped language

- **WHEN** a user configures startup arguments for an interpreter-shaped language
- **THEN** those arguments SHALL be appended after the resolved template rather than replacing it
- **AND** the template itself SHALL NOT be user-configurable, because a template a user can replace is one they can replace with something that does not start the server

### Requirement: A manual override means what the launch shape says it means

A manual override for an executable-shaped language SHALL remain an absolute path to an executable file. A manual override for an interpreter-shaped language SHALL be an absolute path to the server's install directory, and SHALL be validated by the presence of the artifact its argument template requires rather than by executability.

#### Scenario: An interpreter-shaped override names a directory

- **WHEN** a user configures an absolute directory that contains the artifact the language's template requires
- **THEN** discovery SHALL report the server as available
- **AND** it SHALL report the resolved artifact rather than the directory alone

#### Scenario: An interpreter-shaped override names a directory without the artifact

- **WHEN** a configured directory exists but does not contain the required artifact
- **THEN** discovery SHALL report unavailable with a reason distinguishing this from a missing directory
- **AND** no server SHALL be started

#### Scenario: An interpreter-shaped language has no override

- **WHEN** an interpreter-shaped language is enabled with no configured install directory
- **THEN** discovery SHALL report unavailable with a reason saying the install directory is not set
- **AND** it SHALL NOT search the executable path for the server, because the server is not an executable

### Requirement: A versioned launcher artifact is resolved by bounded glob

Where an argument template names an artifact whose file name carries a version, the system SHALL resolve it by matching a declared pattern within a single declared directory of the install, SHALL NOT recurse, and SHALL refuse rather than choose when the match is not unique.

#### Scenario: Exactly one launcher matches

- **WHEN** the declared directory contains exactly one file matching the pattern
- **THEN** that file SHALL be used
- **AND** the resolved path SHALL be reported so a reader can tell which version will run

#### Scenario: No launcher matches

- **WHEN** no file in the declared directory matches the pattern
- **THEN** the launch SHALL be refused with a reason saying the launcher was not found

#### Scenario: Several launchers match

- **WHEN** more than one file matches the pattern
- **THEN** the launch SHALL be refused rather than selecting one
- **AND** the reason SHALL say the install is ambiguous, because an install holding two launchers is not the install the settings page describes

### Requirement: A server prerequisite is reported separately from the server

Where a registered language's server requires a host runtime it does not ship, the system SHALL detect that runtime separately and SHALL report its absence as its own reason rather than as a server-start failure.

#### Scenario: The prerequisite runtime is missing

- **WHEN** an interpreter-shaped language's interpreter cannot be resolved
- **THEN** discovery SHALL report unavailable with a prerequisite reason naming the runtime
- **AND** that reason SHALL be distinct from the reasons for a missing install directory and for a missing launcher

#### Scenario: The prerequisite is present but the server is not

- **WHEN** the interpreter resolves and the install directory is not configured
- **THEN** the reported reason SHALL be the missing install directory, not the prerequisite

### Requirement: Per-workspace server state is removed when trust is revoked

Where a registered language's server is given a writable per-workspace data directory, the system SHALL derive that directory from the canonical workspace so two workspaces never share one, and SHALL remove it when trust for that workspace is revoked.

#### Scenario: Two workspaces are served

- **WHEN** two trusted workspaces are served by the same interpreter-shaped language
- **THEN** each SHALL be given its own data directory
- **AND** neither SHALL be able to read the other's

#### Scenario: Trust is revoked

- **WHEN** trust is revoked for a workspace that has a server data directory
- **THEN** that directory SHALL be removed along with the running process
- **AND** a revoked workspace SHALL NOT leave a server-built index of its source on disk

### Requirement: Java is a registered language

The registry SHALL declare Java served by `jdtls` through a JVM interpreter over stdio, with `pom.xml`, `build.gradle`, `build.gradle.kts`, and `settings.gradle` as project-root markers and `.java` mapped to the `java` language identifier. It SHALL default to disabled and SHALL require workspace trust exactly as the languages registered before it.

#### Scenario: A Java source file is admitted

- **WHEN** a trusted workspace makes a semantic request for a `.java` file
- **THEN** the document SHALL be admitted with the `java` language identifier and routed to the Java server

#### Scenario: Java is disabled on an existing installation

- **WHEN** an installation that predates Java starts a build that registers it
- **THEN** the Java switch SHALL read as disabled
- **AND** no server SHALL start for it until a user enables it and trusts a workspace

#### Scenario: A Gradle project root is detected

- **WHEN** a `.java` file's nearest ancestor holding any Java marker holds `build.gradle.kts`
- **THEN** that directory SHALL be the project root
- **AND** a directory holding several Java markers SHALL resolve to that same directory

## MODIFIED Requirements

### Requirement: Registered language servers are discoverable and testable
The desktop runtime SHALL discover each enabled registered language's declared server, and SHALL test a discovered server through an isolated bounded initialize and shutdown lifecycle without opening an interactive session. For an executable-shaped language, discovery SHALL resolve the declared executables in the registry's preference order from a configured absolute override or the native executable search path. For an interpreter-shaped language, discovery SHALL resolve the interpreter and the configured install directory instead. Each registered language SHALL declare the startup arguments its server requires, and every currently registered language SHALL communicate over stdio.

#### Scenario: Executable is automatically discovered
- **WHEN** an enabled executable-shaped server has no manual override and a supported executable exists on the native search path
- **THEN** discovery SHALL report the resolved server kind and an available status
- **AND** discovery SHALL NOT start a persistent workspace server

#### Scenario: Manual override is unavailable
- **WHEN** a configured override does not resolve to what its launch shape requires
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

### Requirement: Supported languages are defined by a language registry
The system SHALL define every supported language as a registry entry carrying a stable language id, its launch shape, candidate server executable names in preference order or an interpreter and argument template, project-root markers, source-file extension to LSP language identifier mapping, default startup arguments, default initialization options, and platform applicability. Configuration, discovery, project-root detection, document admission, server testing, and the settings surface SHALL derive their supported-language set from that registry. Persisted configuration SHALL accept any registered language id, and the storage layer SHALL NOT constrain which language ids may exist.

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
