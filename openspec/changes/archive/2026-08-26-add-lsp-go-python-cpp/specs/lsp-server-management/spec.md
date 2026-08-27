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

## MODIFIED Requirements

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
