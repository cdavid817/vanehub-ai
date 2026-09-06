## MODIFIED Requirements

### Requirement: Stable folder-opener catalog
The desktop runtime SHALL expose stable opener ids for `vscode`, `file-explorer`, `windows-terminal`, `git-bash`, `intellij-idea`, and `webstorm`, with localized display metadata, category, icon identity, availability status, and resolved installation details when available.

#### Scenario: List supported Windows openers
- **WHEN** the frontend requests folder-opener availability on Windows, macOS, or Linux
- **THEN** the service SHALL return one deterministic entry for each supported opener id
- **AND** each entry SHALL distinguish available, not installed, invalid installation, unsupported platform, and detection failure states without launching the program

#### Scenario: Report platform support honestly
- **WHEN** the host platform is macOS or Linux
- **THEN** `windows-terminal` and `git-bash` SHALL report `unsupported-platform`
- **AND** `vscode`, `file-explorer`, `intellij-idea`, and `webstorm` SHALL report `available` or `not-installed` according to discovery, never `unsupported-platform`

#### Scenario: Preserve stable identity across installation changes
- **WHEN** a supported program is upgraded, moved, uninstalled, or reinstalled
- **THEN** the opener id SHALL remain stable while its runtime availability and resolved details MAY change

### Requirement: Bounded deterministic application discovery
The desktop runtime SHALL discover supported programs through bounded product-specific sources on every supported platform and SHALL NOT recursively scan arbitrary drives or start an interactive program as part of discovery.

#### Scenario: Detect a custom or registered installation
- **WHEN** a supported program is absent from `PATH` but has a valid App Paths, uninstall registry, known product, Git for Windows (system or user scope), JetBrains Toolbox, JetBrains Toolbox 2.x `%LOCALAPPDATA%\Programs`, Scoop, or Store app-execution-alias location
- **THEN** discovery SHALL validate and report that installation according to deterministic source and version ranking

#### Scenario: Detect a JetBrains Toolbox installation on Linux
- **WHEN** IntelliJ IDEA or WebStorm is installed through JetBrains Toolbox under the user's local data directory, or as a `/opt`, snap, or Flatpak package
- **THEN** discovery SHALL report the JetBrains-managed launcher or the package's export wrapper with its resolved path
- **AND** SHALL prefer the Toolbox-managed launcher over a `PATH` wrapper when both exist

#### Scenario: Detect an application bundle on macOS
- **WHEN** Visual Studio Code, IntelliJ IDEA, or WebStorm is installed as an application bundle in `/Applications` or `~/Applications`, or through JetBrains Toolbox in either its current or channel-directory layout
- **THEN** discovery SHALL report the bundle or Toolbox launcher without starting it
- **AND** a CLI wrapper in `/usr/local/bin` or `/opt/homebrew/bin` SHALL be found even when the application's own `PATH` omits those directories

#### Scenario: Resolve the system file manager off Windows
- **WHEN** the host platform is Linux or macOS
- **THEN** `file-explorer` SHALL resolve to `xdg-open` or `/usr/bin/open` respectively and report `not-installed` when that launcher is absent

#### Scenario: Surface JetBrains product metadata
- **WHEN** a discovered JetBrains launcher has a readable `product-info.json`
- **THEN** discovery SHALL report the product version and edition from it
- **AND** an unreadable or missing file SHALL leave both fields empty without failing discovery

#### Scenario: Avoid WSL Bash misclassification
- **WHEN** Windows or WSL exposes `bash.exe` but no validated Git for Windows `git-bash.exe` exists
- **THEN** discovery SHALL NOT report Git Bash as available

#### Scenario: Handle one detector failure
- **WHEN** one product source is unreadable or malformed
- **THEN** discovery SHALL return a safe failure or alternate-source result for that opener
- **AND** SHALL continue returning results for the remaining catalog

#### Scenario: Keep discovery off the UI thread
- **WHEN** the frontend lists, refreshes, or launches through a native command
- **THEN** the command SHALL run discovery and launch on a blocking worker rather than the main thread
- **AND** a launch SHALL reuse the cached catalog and only re-run discovery when the cached launcher no longer exists

### Requirement: Safe detached external launch
The native runtime SHALL launch supported external programs through a fixed product-specific, platform-specific plan with an explicit executable, argument vector, and working directory, without shell command concatenation, and SHALL treat OS spawn acceptance as completion.

#### Scenario: Launch a supported opener
- **WHEN** the selected opener and resolved session directory remain valid at launch time
- **THEN** the runtime SHALL start the allowlisted program detached with the directory represented as an explicit argument or working directory
- **AND** SHALL return without waiting for the external program to exit

#### Scenario: Launch a macOS application bundle
- **WHEN** the discovered executable is a macOS application bundle
- **THEN** the launch plan SHALL invoke `/usr/bin/open` with the bundle and the directory as separate arguments

#### Scenario: Launch the system file manager off Windows
- **WHEN** `file-explorer` is launched on Linux or macOS
- **THEN** the plan SHALL invoke `xdg-open` or `/usr/bin/open` with the directory as its one argument

#### Scenario: Preserve a special-character path as data
- **WHEN** the target directory contains spaces, shell metacharacters, Unicode, or command-like text
- **THEN** the entire directory SHALL remain a literal process argument or working directory
- **AND** no shell SHALL interpret any part of it

#### Scenario: Survive a parent interrupt on Unix
- **WHEN** an opener is started on Linux or macOS
- **THEN** the child SHALL be placed in its own process group so a signal delivered to the application's group does not terminate it

#### Scenario: Detect an installation disappearing before launch
- **WHEN** the previously discovered executable is no longer valid at launch time
- **THEN** the operation SHALL fail safely and mark or refresh the opener availability without invoking a fallback program silently

### Requirement: Runtime adapter parity
The frontend service boundary SHALL expose opener availability, refresh, preference read/write, and session-folder launch operations through both desktop and Web/mock adapters.

#### Scenario: Use desktop folder openers
- **WHEN** the Tauri adapter receives an opener request
- **THEN** it SHALL route through declared native commands and return the service contract result

#### Scenario: Preview folder openers on the Web
- **WHEN** the Web/mock runtime lists or configures folder openers
- **THEN** it SHALL return deterministic catalog and preference data
- **AND** a launch request SHALL report native action unavailable without claiming a local process was started

#### Scenario: Notify other surfaces after a refresh
- **WHEN** a refresh completes in either adapter
- **THEN** subscribers of folder-opener events SHALL be notified so the session toolbar reloads the catalog
- **AND** the notification SHALL NOT change saved preferences
