# lsp-server-management Specification

## Purpose
Defines safe, observable, and bounded discovery, trust, process, protocol, document, and shutdown behavior for language servers attached to local VaneHub workspaces.
## Requirements
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

### Requirement: Server instances are scoped to detected project roots
The system SHALL key a language-server instance by canonical session workspace, bounded detected project root, server kind, and configuration fingerprint. Root detection SHALL choose the nearest supported project marker without traversing above the session workspace. A language MAY require that a marker be found, in which case detection SHALL fail rather than fall back to the session workspace root.

#### Scenario: Workspace has two TypeScript project roots
- **WHEN** two queried files resolve to distinct nested TypeScript project roots in one session workspace
- **THEN** the system SHALL route them to independently keyed server instances

#### Scenario: Project marker is outside the workspace
- **WHEN** upward root detection would reach a project marker above the canonical session workspace
- **THEN** the system SHALL stop at the session workspace boundary
- **AND** it SHALL NOT expose or use the outside marker

#### Scenario: Configuration changes while a server is running
- **WHEN** the executable, startup arguments, initialization options, or trust revision changes
- **THEN** the old configuration fingerprint SHALL become stale
- **AND** matching server instances SHALL drain and restart before serving requests under the new configuration

#### Scenario: Two languages share one project root
- **WHEN** files of two registered languages resolve to the same detected project root in one session workspace
- **THEN** the system SHALL key an independent server instance per language
- **AND** neither instance SHALL receive the other language's documents

### Requirement: LSP transport is bounded bidirectional JSON-RPC
The desktop runtime SHALL support requests, responses, notifications, and server-to-client requests through JSON-RPC 2.0 with LSP Content-Length framing over child stdin/stdout. Frame sizes, headers, stderr capture, queues, pending requests, and concurrent requests SHALL have hard bounds, and responses SHALL be correlated by request id even when they arrive out of order.

#### Scenario: Two responses arrive out of order
- **WHEN** two concurrent LSP requests receive responses in reverse order
- **THEN** each caller SHALL receive the response matching its own request id

#### Scenario: Server requests workspace configuration
- **WHEN** a server sends a supported `workspace/configuration` or capability-registration request
- **THEN** the client SHALL return the bounded configuration or registration result required by its declared capabilities
- **AND** it SHALL continue processing unrelated pending requests

#### Scenario: Server requests a workspace edit
- **WHEN** a server sends `workspace/applyEdit` during this read-only foundation
- **THEN** the client SHALL reject the edit without changing any file

#### Scenario: Protocol frame exceeds its limit
- **WHEN** stdout declares or produces a frame beyond the configured hard limit
- **THEN** the system SHALL fail pending requests with a protocol-limit status
- **AND** it SHALL terminate the offending managed process

### Requirement: Requests are timed out and cancellable
Every LSP request SHALL have a fixed bounded deadline and SHALL observe the owning Agent-generation cancellation signal. Cancellation of a real pending request SHALL send `$/cancelRequest` with that request id when the transport remains available, and timeout or cancellation SHALL release local pending state.

#### Scenario: Definition request times out
- **WHEN** a server does not answer a definition request before its deadline
- **THEN** the request SHALL return a timeout status
- **AND** it SHALL NOT remain in the pending-request map

#### Scenario: Agent generation is cancelled
- **WHEN** the user cancels an Agent generation with an in-flight LSP request
- **THEN** the request SHALL stop waiting, attempt protocol cancellation for its actual id, and return through bounded cleanup

### Requirement: Server capabilities are negotiated before use
The client SHALL advertise only implemented capabilities, complete `initialize` followed by `initialized`, record the selected position encoding and text-document synchronization mode, and record which of the semantic methods it implements the server advertises. It SHALL issue a semantic request only when that record reports support for the method. The record SHALL be a list of negotiated methods rather than a fixed set of fields, so a method added to the client appears in it without any consumer being told the method's name in advance. Protocol readiness SHALL remain distinct from optional background indexing progress.

#### Scenario: Server selects no position encoding
- **WHEN** the initialize result omits a selected position encoding
- **THEN** the client SHALL use UTF-16 position semantics

#### Scenario: Hover is unsupported
- **WHEN** a configured server reports no hover capability
- **THEN** a hover query SHALL return an unavailable status without sending an unsupported request

#### Scenario: Server reports indexing progress
- **WHEN** a server publishes work-done progress after initialization
- **THEN** server status SHALL expose bounded warming or indexing detail
- **AND** protocol-ready requests SHALL remain eligible to run

#### Scenario: A server advertises a method the client does not implement
- **WHEN** an initialize result advertises a capability outside the set of methods the client implements
- **THEN** the negotiated record SHALL omit it
- **AND** the client SHALL NOT report it as available anywhere

#### Scenario: A method the client implements is absent from the initialize result
- **WHEN** an initialize result omits a capability for a method the client implements
- **THEN** the negotiated record SHALL report that method as unsupported rather than omitting it
- **AND** a request for it SHALL return unavailable without being sent

#### Scenario: Negotiated methods are reported in a stable order
- **WHEN** two servers negotiate the same set of methods
- **THEN** their negotiated records SHALL list those methods in the same order
- **AND** a consumer rendering the list SHALL NOT have to sort it to be deterministic

### Requirement: Disk content is the authoritative document state
Before a text-document operation, the system SHALL resolve a bounded UTF-8 disk snapshot inside the current canonical workspace, send `didOpen` for a new document lease, and send a versioned full or incremental `didChange` matching the negotiated synchronization mode when the disk content changes. It SHALL send `didClose` when a lease expires or its server stops.

#### Scenario: First request opens a document
- **WHEN** a trusted workspace makes its first semantic request for an admitted source file
- **THEN** the client SHALL send `didOpen` with the current disk content, language id, and document version before the request

#### Scenario: Agent changes an open document
- **WHEN** a successful Agent file mutation targets a document with an active lease
- **THEN** the lease SHALL be invalidated immediately
- **AND** the next semantic operation SHALL synchronize the new disk content before issuing its request

#### Scenario: External editor changes a document
- **WHEN** disk content changes without an Agent mutation signal
- **THEN** the next semantic operation SHALL detect the changed snapshot and increment the document version before querying the server

#### Scenario: Document target is unsafe
- **WHEN** a requested document is outside the canonical workspace, hidden, binary, oversized, non-file, or reached through an escaping symbolic link
- **THEN** the client SHALL reject it without sending its content to a language server

### Requirement: Server lifecycle is bounded and observable
The system SHALL expose absent, starting, initializing, ready, stopping, backoff, and failed states with safe server identity, project root display, restart count, last response time, diagnostic count, and negotiated-capability summary. Unexpected exits SHALL use bounded restart backoff, idle servers SHALL close after the configured timeout, and desktop shutdown SHALL attempt `shutdown` then `exit` before forcibly terminating remaining process trees.

#### Scenario: Server crashes repeatedly
- **WHEN** a server exceeds its restart budget during the cooldown window
- **THEN** it SHALL enter failed state and stop restarting automatically
- **AND** status SHALL expose a safe restart-exhausted reason

#### Scenario: Server becomes idle
- **WHEN** a server has no active request or document lease for ten minutes
- **THEN** the system SHALL initiate graceful shutdown and remove the exited instance from the active pool

#### Scenario: Application exits with running servers
- **WHEN** desktop shutdown begins while one or more language servers are running
- **THEN** the system SHALL stop accepting new requests, attempt graceful protocol shutdown concurrently under a global deadline, and terminate any remaining process trees

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

### Requirement: Go, Python, and C/C++ are registered languages
The registry SHALL declare Go served by `gopls` over stdio with no startup arguments, Python served by `basedpyright-langserver` or `pyright-langserver` over stdio with `--stdio`, and C/C++ served by `clangd` over stdio with no startup arguments. Each SHALL declare its own project-root markers, source-file extension to LSP language identifier mappings, and isolated server-test fixture project. All three SHALL default to disabled and SHALL require workspace trust exactly as the languages registered before them.

#### Scenario: A newly registered language is disabled on an existing installation
- **WHEN** an installation that predates these languages starts a build that registers them
- **THEN** each new language switch SHALL read as disabled
- **AND** no server SHALL start for it until a user enables it and trusts a workspace

#### Scenario: Both Python servers are installed
- **WHEN** `basedpyright-langserver` and `pyright-langserver` both resolve on the native search path
- **THEN** discovery SHALL select `basedpyright-langserver`
- **AND** it SHALL report which candidate was selected

#### Scenario: Only the upstream Python server is installed
- **WHEN** `basedpyright-langserver` does not resolve and `pyright-langserver` does
- **THEN** discovery SHALL select `pyright-langserver` and report an available status

#### Scenario: A Go source file is admitted
- **WHEN** a trusted workspace makes a semantic request for a `.go` file
- **THEN** the document SHALL be admitted with the `go` language identifier and routed to the Go server

### Requirement: A language may declare several project-root markers
A registered language MAY declare more than one project-root marker, and any one of them SHALL identify a project root on its own. Detection SHALL choose the nearest ancestor directory holding any of that language's markers; a directory holding several of them SHALL resolve to that same directory. A marker MAY name a path inside the candidate directory rather than a file directly in it.

#### Scenario: A nearer directory holds a different marker than a further one
- **WHEN** the nearest ancestor holds one of the language's markers and a further ancestor holds another
- **THEN** detection SHALL choose the nearer directory
- **AND** which marker each directory holds SHALL NOT change that

#### Scenario: One directory holds several markers
- **WHEN** a candidate directory holds more than one of the language's declared markers
- **THEN** detection SHALL resolve that directory once
- **AND** the result SHALL be identical to the result for a directory holding only one of them

#### Scenario: Every Python marker identifies a root on its own
- **WHEN** a Python project root directory holds only `pyproject.toml`, only `setup.py`, only `setup.cfg`, or only `requirements.txt`
- **THEN** detection SHALL resolve that directory as the project root in each case

#### Scenario: A marker names a nested path
- **WHEN** a language declares a marker containing a path separator and a candidate directory holds that relative path as a file
- **THEN** detection SHALL resolve the candidate directory, not the directory the file sits in

### Requirement: A C or C++ project root is a compilation database
C/C++ root detection SHALL locate the nearest ancestor directory containing a `compile_commands.json`, or a `build` subdirectory containing one, without traversing above the canonical session workspace. When no compilation database exists within the workspace, the system SHALL report an unavailable outcome with a safe reason distinct from a general project-root failure, and SHALL NOT start a server for that request. The outcome SHALL carry the language identity so the missing marker can be attributed to C/C++ rather than guessed at.

#### Scenario: A workspace has no compilation database
- **WHEN** a semantic request targets a C or C++ file in a workspace containing no `compile_commands.json`
- **THEN** the outcome SHALL be unavailable with the missing-project-marker reason and the C/C++ language identity
- **AND** it SHALL NOT report the generic not-configured reason, which would send a user to the settings page instead of to their build system
- **AND** no `clangd` process SHALL start

#### Scenario: The compilation database is in a build directory
- **WHEN** the nearest ancestor directory contains `build/compile_commands.json` rather than `compile_commands.json`
- **THEN** that ancestor SHALL be the detected project root

#### Scenario: The compilation database is outside the workspace
- **WHEN** the only `compile_commands.json` reachable by upward traversal is above the canonical session workspace
- **THEN** detection SHALL stop at the workspace boundary and report the same missing-project-marker outcome
- **AND** it SHALL NOT expose or use the outside file

#### Scenario: An installed server does not imply a usable one
- **WHEN** `clangd` is discovered as available but the queried workspace has no compilation database
- **THEN** discovery SHALL continue to report the server as available
- **AND** the per-request outcome SHALL still be the missing-project-marker unavailable result

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

A manual override for an executable-shaped language SHALL remain an absolute path to an executable file. A manual override for an interpreter-shaped language SHALL be an absolute path to the server's install directory, and SHALL be validated by the presence of the artifact its argument template requires rather than by executability. Where no override is configured and a managed install exists, discovery SHALL use the managed install; an override SHALL always take precedence over one.

#### Scenario: An interpreter-shaped override names a directory

- **WHEN** a user configures an absolute directory that contains the artifact the language's template requires
- **THEN** discovery SHALL report the server as available
- **AND** it SHALL report the resolved artifact rather than the directory alone

#### Scenario: An interpreter-shaped override names a directory without the artifact

- **WHEN** a configured directory exists but does not contain the required artifact
- **THEN** discovery SHALL report unavailable with a reason distinguishing this from a missing directory
- **AND** no server SHALL be started

#### Scenario: An interpreter-shaped language has no override

- **WHEN** an interpreter-shaped language is enabled with no configured install directory and no managed install
- **THEN** discovery SHALL report unavailable with a reason saying the install directory is not set
- **AND** it SHALL NOT search the executable path for the server, because the server is not an executable

#### Scenario: A managed install is used when no override is set

- **WHEN** an interpreter-shaped language has a managed install and no configured override
- **THEN** discovery SHALL resolve the server from the managed install
- **AND** it SHALL report the resolved artifact, so a reader can tell which version will run

#### Scenario: An override is set alongside a managed install

- **WHEN** both a manual override and a managed install exist for the same language
- **THEN** discovery SHALL use the override
- **AND** the managed install SHALL be left in place rather than removed, because the user may switch back

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

### Requirement: A language may declare a published distribution

A registered language MAY declare where its server is published: an allowlisted host, a URL, an integrity expectation, and extraction limits. The system SHALL acquire it through `managed-tool-installation` rather than through any download path of its own, and SHALL install it into a directory VaneHub owns.

#### Scenario: A declared server is installed

- **WHEN** a user installs a language that declares a distribution
- **THEN** the artifact SHALL be retrieved and extracted under the shared capability's bounds
- **AND** the finished install SHALL be placed only after extraction completes, so an interrupted install never leaves a directory that looks installed

#### Scenario: Installation is refused

- **WHEN** the shared retrieval or extraction refuses the artifact
- **THEN** the install SHALL fail with that reason
- **AND** no partially extracted directory SHALL remain, and the language SHALL still report as not installed

#### Scenario: A declared artifact publishes no digest

- **WHEN** a language declares a distribution with no published digest
- **THEN** the download SHALL still apply the host allowlist, the byte ceiling, the deadline, and cancellation
- **AND** the surface offering the install SHALL state that the bytes are not verified, rather than presenting an unverified download as a verified one

#### Scenario: A language declares no distribution

- **WHEN** a registered language declares no published distribution
- **THEN** no install action SHALL be offered for it
- **AND** its discovery SHALL behave exactly as it did before this capability existed

### Requirement: A managed install is removable and never confused with the user's own

The system SHALL remove only the install directory it created, SHALL leave a manually configured directory untouched, and SHALL report the language as not installed once its managed directory is gone.

#### Scenario: A managed install is removed

- **WHEN** a user uninstalls a language whose server VaneHub installed
- **THEN** the managed directory SHALL be removed
- **AND** any running server for that language SHALL be stopped first, because a running server holds files in that directory open

#### Scenario: The user pointed at their own copy

- **WHEN** a user uninstalls a language while a manual override names a directory VaneHub did not create
- **THEN** only the managed directory SHALL be removed
- **AND** the directory the override names SHALL be left exactly as it was

