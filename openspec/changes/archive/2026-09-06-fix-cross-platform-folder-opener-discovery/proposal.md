## Why

Folder-opener discovery is compiled as Windows-only. On Linux and macOS every opener, including IntelliJ IDEA and WebStorm installed through JetBrains Toolbox, is reported as unavailable, so the Basic Configuration workspace group shows an empty default-opener select and the session toolbar's open action stays disabled. A user with IDEA Ultimate installed under `~/.local/share/JetBrains/Toolbox/apps/` sees it listed as not installed. Windows discovery also misses the JetBrains Toolbox 2.x install root (`%LOCALAPPDATA%\Programs`), so an up-to-date Toolbox installation is reported missing there too.

The settings section that presents this data has three presentation defects on top of the data problem: the default-opener select renders blank when nothing is available, availability is shown as unstyled muted text so an installed program and an unsupported one look the same at a glance, and opener management is a framed disclosure nested inside a framed settings panel, which the project's visual rules forbid.

## What Changes

- Discover openers on Linux and macOS through bounded product-specific locations: `PATH`, JetBrains Toolbox scripts and app roots, `/opt`, snap, and macOS application bundles. Report `not-installed` for a supported-but-absent program and reserve `unsupported-platform` for Windows Terminal and Git Bash off Windows.
- Add the JetBrains Toolbox 2.x `%LOCALAPPDATA%\Programs` root and `%ProgramFiles(x86)%\JetBrains` to Windows discovery.
- Read `product-info.json` beside a discovered JetBrains launcher to surface version and edition without starting the program.
- Build the launch plan per platform: `xdg-open` and `open` for the system file manager, `open -a` for macOS application bundles, and the launcher script or binary elsewhere; the directory stays a literal argument on every platform.
- Show a localized empty state in the default-opener select when no enabled opener is available, present availability with the shared status pill, and render opener management as an embedded disclosure instead of a nested card.
- Use the shared settings page header on Basic Configuration so it matches the other settings pages.
- Run the three opener commands on a blocking worker instead of the main thread, reuse the cached catalog at launch time, and notify the session toolbar after a refresh.
- Give directory settings a native browse action, Enter-to-commit, a saved acknowledgement, and native validation that the default folder exists; report the startup capability as an explicit flag; make reset attempt every key and report all failures.
- Replace raw pixel font-size labels and the hard-coded theme branch with registry-driven localized labels, drop the developer-facing storage notes, and add a re-detect action to the Node.js panel.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `workspace-folder-openers`: Extend discovery, status semantics, and the launch plan to Linux and macOS, and add the Toolbox 2.x Windows root.
- `settings-basic-configuration-ui`: Require an explicit empty state for the default-opener select, status-pill availability presentation, an unframed opener-management disclosure, browse/Enter/saved behavior for directory fields, human option labels, no developer notes, and a reset that finishes every key.
- `app-settings`: Add native validation of the default folder path, a directory-picker service operation, and an explicit read-only startup capability flag.

## Impact

- `src-tauri/src/contexts/desktop/infrastructure/folder_openers.rs` and a new discovery module beside it; `src-tauri/src/platform/process/mod.rs` gains a Unix process-group detach.
- `src/settings/pages/folder-openers-section.tsx`, `src/settings/pages/page-parts.tsx`, `src/settings/pages/basic-settings-page.tsx`, and synchronized locale resources for the new empty-state text.
- No schema change, no new dependency, no service-contract change: the `FolderOpenerAvailability` shape is unchanged and the Web/mock adapter keeps its deterministic fixture.
