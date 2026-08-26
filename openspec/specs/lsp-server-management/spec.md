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
The system SHALL key a language-server instance by canonical session workspace, bounded detected project root, server kind, and configuration fingerprint. Root detection SHALL choose the nearest supported project marker without traversing above the session workspace.

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
The client SHALL advertise only implemented capabilities, complete `initialize` followed by `initialized`, record the selected position encoding and text-document synchronization mode, and issue a semantic request only when the server reports support for that method. Protocol readiness SHALL remain distinct from optional background indexing progress.

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

