# workspace-folder-openers Specification

## Purpose
Defines the stable folder-opener catalog, bounded Windows application discovery, preference invariants, authorized session-folder resolution, safe detached launch, diagnostics, and runtime adapter parity.

## Requirements

### Requirement: Stable folder-opener catalog
The desktop runtime SHALL expose stable opener ids for `vscode`, `file-explorer`, `windows-terminal`, `git-bash`, `intellij-idea`, and `webstorm`, with localized display metadata, category, icon identity, availability status, and resolved installation details when available.

#### Scenario: List supported Windows openers
- **WHEN** the frontend requests folder-opener availability on Windows
- **THEN** the service SHALL return one deterministic entry for each supported opener id
- **AND** each entry SHALL distinguish available, not installed, invalid installation, unsupported platform, and detection failure states without launching the program

#### Scenario: Preserve stable identity across installation changes
- **WHEN** a supported program is upgraded, moved, uninstalled, or reinstalled
- **THEN** the opener id SHALL remain stable while its runtime availability and resolved details MAY change

### Requirement: Bounded deterministic application discovery
The desktop runtime SHALL discover supported programs through bounded product-specific sources and SHALL NOT recursively scan arbitrary drives or start an interactive program as part of discovery.

#### Scenario: Detect a custom or registered installation
- **WHEN** a supported program is absent from `PATH` but has a valid App Paths, uninstall registry, known product, Git for Windows, or JetBrains Toolbox location
- **THEN** discovery SHALL validate and report that installation according to deterministic source and version ranking

#### Scenario: Avoid WSL Bash misclassification
- **WHEN** Windows or WSL exposes `bash.exe` but no validated Git for Windows `git-bash.exe` exists
- **THEN** discovery SHALL NOT report Git Bash as available

#### Scenario: Handle one detector failure
- **WHEN** one product source is unreadable or malformed
- **THEN** discovery SHALL return a safe failure or alternate-source result for that opener
- **AND** SHALL continue returning results for the remaining catalog

### Requirement: Folder-opener preference invariants
The system SHALL manage one configured default opener and an ordered, duplicate-free enabled opener list using only supported stable ids, and SHALL keep File Explorer enabled as the Windows fallback.

#### Scenario: Save valid preferences
- **WHEN** the user saves an available enabled opener as the configured default with one or more enabled ids
- **THEN** the system SHALL validate and atomically persist the complete preference aggregate

#### Scenario: Reject invalid preferences
- **WHEN** preferences contain an unknown id, omit File Explorer, or select a default outside the enabled list
- **THEN** the system SHALL reject the mutation without partially changing persisted preferences

#### Scenario: Retain an unavailable enabled opener
- **WHEN** a non-default enabled program becomes unavailable
- **THEN** the system SHALL retain the user's enabled selection while excluding it from executable actions until rediscovered

### Requirement: Effective default fallback
The system SHALL distinguish the configured default from the effective default and SHALL compute a usable fallback without overwriting user intent.

#### Scenario: Use the configured default
- **WHEN** the configured default is enabled and available
- **THEN** it SHALL be returned as the effective default with fallback inactive

#### Scenario: Fall back after an uninstall
- **WHEN** the configured default is unavailable
- **THEN** File Explorer SHALL become the effective default when available
- **AND** the configured default SHALL remain persisted
- **AND** the result SHALL identify that fallback is active

### Requirement: Authorized session-folder resolution
The native launch operation SHALL accept a session id and supported opener id, resolve an authorized local directory in worktree, folder, then project order, and SHALL NOT accept an arbitrary frontend executable or target path.

#### Scenario: Open a worktree session
- **WHEN** a local session has worktree, folder, and project paths and the user requests an available opener
- **THEN** the native runtime SHALL validate and open the worktree directory

#### Scenario: Fall back through local path fields
- **WHEN** a local session has no worktree path but has a folder or project path
- **THEN** the native runtime SHALL select the first existing directory in folder then project order

#### Scenario: Reject an unavailable target
- **WHEN** the session has no authorized local root or the resolved directory no longer exists
- **THEN** the operation SHALL return a concise unavailable error without starting a process

#### Scenario: Reject a remote workspace
- **WHEN** the selected session represents a remote workspace
- **THEN** the operation SHALL return a concise unsupported result without interpreting its remote path as a local directory

### Requirement: Safe detached external launch
The native runtime SHALL launch supported external programs through a fixed product-specific plan with an explicit executable, argument vector, and working directory, without shell command concatenation, and SHALL treat OS spawn acceptance as completion.

#### Scenario: Launch a supported opener
- **WHEN** the selected opener and resolved session directory remain valid at launch time
- **THEN** the runtime SHALL start the allowlisted program detached with the directory represented as an explicit argument or working directory
- **AND** SHALL return without waiting for the external program to exit

#### Scenario: Preserve a special-character path as data
- **WHEN** the target directory contains spaces, shell metacharacters, Unicode, or command-like text
- **THEN** the entire directory SHALL remain a literal process argument or working directory
- **AND** no shell SHALL interpret any part of it

#### Scenario: Detect an installation disappearing before launch
- **WHEN** the previously discovered executable is no longer valid at launch time
- **THEN** the operation SHALL fail safely and mark or refresh the opener availability without invoking a fallback program silently

### Requirement: Folder-opener diagnostics
Native discovery and launch failures SHALL use unified redacted logging with safe opener, session, source, target-kind, result, and error-code context while avoiding raw workspace and executable paths in normal records.

#### Scenario: Persist a launch failure
- **WHEN** a supported opener cannot be started
- **THEN** the runtime SHALL write a redacted unified diagnostic with the opener id and safe failure classification
- **AND** the frontend SHALL receive a concise localized error

### Requirement: Runtime adapter parity
The frontend service boundary SHALL expose opener availability, refresh, preference read/write, and session-folder launch operations through both desktop and Web/mock adapters.

#### Scenario: Use desktop folder openers
- **WHEN** the Tauri adapter receives an opener request
- **THEN** it SHALL route through declared native commands and return the service contract result

#### Scenario: Preview folder openers on the Web
- **WHEN** the Web/mock runtime lists or configures folder openers
- **THEN** it SHALL return deterministic catalog and preference data
- **AND** a launch request SHALL report native action unavailable without claiming a local process was started
