## ADDED Requirements

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

### Requirement: Root markers are ordered by declared strength
A registered language MAY declare more than one project-root marker. Root detection SHALL still choose the nearest ancestor directory containing any of that language's markers, and SHALL resolve a directory containing several of them by the registry's declared marker order. The declared order SHALL express which marker more strongly identifies a project root, not the order in which they were added.

#### Scenario: One directory holds two markers of different strength
- **WHEN** a candidate directory contains both a stronger and a weaker declared marker for the queried language
- **THEN** root detection SHALL resolve that directory as the project root
- **AND** the stronger marker SHALL be the one reported as the reason

#### Scenario: A nearer directory holds only the weaker marker
- **WHEN** the nearest ancestor holds only a weaker marker and a further ancestor holds a stronger one
- **THEN** root detection SHALL choose the nearer directory
- **AND** proximity SHALL take precedence over marker strength

#### Scenario: Python markers are ranked
- **WHEN** a Python project root is detected
- **THEN** `pyproject.toml` SHALL rank above `setup.py`, which SHALL rank above `setup.cfg`, which SHALL rank above `requirements.txt`

### Requirement: A C or C++ project root is a compilation database
C/C++ root detection SHALL locate the nearest ancestor directory containing a `compile_commands.json`, or a `build` subdirectory containing one, without traversing above the canonical session workspace. When no compilation database exists within the workspace, the system SHALL report an unavailable outcome naming the missing compilation database and SHALL NOT start a server for that request.

#### Scenario: A workspace has no compilation database
- **WHEN** a semantic request targets a C or C++ file in a workspace containing no `compile_commands.json`
- **THEN** the outcome SHALL be unavailable with a safe reason identifying the missing compilation database
- **AND** no `clangd` process SHALL start

#### Scenario: The compilation database is in a build directory
- **WHEN** the nearest ancestor directory contains `build/compile_commands.json` rather than `compile_commands.json`
- **THEN** that ancestor SHALL be the detected project root

#### Scenario: The compilation database is outside the workspace
- **WHEN** the only `compile_commands.json` reachable by upward traversal is above the canonical session workspace
- **THEN** detection SHALL stop at the workspace boundary and report the same missing-compilation-database outcome
- **AND** it SHALL NOT expose or use the outside file

#### Scenario: An installed server does not imply a usable one
- **WHEN** `clangd` is discovered as available but the queried workspace has no compilation database
- **THEN** discovery SHALL continue to report the server as available
- **AND** the per-request outcome SHALL still be the missing-compilation-database unavailable result

## MODIFIED Requirements

### Requirement: Server instances are scoped to detected project roots
The system SHALL key a language-server instance by canonical session workspace, bounded detected project root, server kind, and configuration fingerprint. Root detection SHALL choose the nearest supported project marker without traversing above the session workspace, resolving a directory that holds several of a language's markers by their declared order.

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
