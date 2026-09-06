## Context

`folder_openers.rs` starts `detect()` with `if !cfg!(windows) { return missing(id) }`, and `missing()` maps every id to `unsupported-platform` off Windows. The spec that introduced the feature (`add-workspace-folder-openers`) was scoped to Windows, so this is not a regression; it is a scope boundary that the product has since crossed by shipping Linux and macOS builds.

## Goals / Non-Goals

**Goals:**

- Detect VS Code, IntelliJ IDEA, WebStorm, and the system file manager on Linux and macOS with the same bounded, no-launch discovery discipline the Windows path already follows.
- Keep the opener catalog, ids, preference invariants, and the frontend contract unchanged.
- Fix the three presentation defects in the settings section without adding a component library or page-local styling.

**Non-Goals:**

- Adding new opener ids (Finder, Nautilus, Dolphin, or per-distribution terminals). `file-explorer` remains the single system file-manager slot and resolves to `xdg-open` or `open`.
- Recursive scans of `/usr`, `/opt`, or `%LOCALAPPDATA%\Programs`. Every root is walked to a fixed shallow depth with a name filter.
- AppImage or hand-extracted tarballs in arbitrary directories. There is no bounded place to look for them; a user with such an install can still reach it through a `PATH` wrapper.

## Decisions

### 1. Split discovery into its own module and make candidate builders take an explicit environment

`folder_opener_discovery.rs` owns `detect()`, platform candidate lists, and `launch_plan()`. Candidate builders take a `DiscoveryEnv` snapshot (home, `PATH` entries, Windows program roots) instead of reading the process environment inline, so unit tests can point them at a temporary directory tree and assert the ranking without touching the host. The service module keeps preferences, caching, launch, and logging.

### 2. Status semantics: `unsupported-platform` means the product does not exist here

Off Windows, `windows-terminal` and `git-bash` are `unsupported-platform`; everything else that is not found is `not-installed`. The frontend already localizes both states, and the session toolbar hides both, so the change is purely in which label the user reads.

### 3. `PATH` lookup walks the variable instead of spawning `which`

On Unix, resolving `code`, `idea`, `webstorm`, and `xdg-open` iterates `PATH` and checks `is_file`. Windows keeps `where` because `PATHEXT` resolution is not worth reimplementing. No discovery step starts an interactive program.

### 3a. Per-platform install channels are enumerated explicitly

Each platform lists the install channels a real user is likely to have, and each channel resolves to a fixed path or a name-filtered shallow walk:

| Platform | Channels |
| --- | --- |
| Windows | App Paths registry, `where`, Toolbox 2.x `%LOCALAPPDATA%\Programs`, Toolbox 1.x `Toolbox\apps`, `Program Files` and `Program Files (x86)`, Scoop `scoop\apps\<name>\current`, Store alias `WindowsApps\wt.exe`, user-scope Git `%LOCALAPPDATA%\Programs\Git`, `%SystemRoot%\explorer.exe` |
| macOS | `/Applications`, `~/Applications`, `~/Applications/JetBrains Toolbox`, Toolbox 1.x `apps/<PRODUCT>/ch-<n>/<build>/<Name>.app`, Toolbox scripts, `PATH` plus `/usr/local/bin` and `/opt/homebrew/bin` because a GUI-launched process has a minimal `PATH` |
| Linux | `PATH` plus `/usr/local/bin`, Toolbox 2.x `apps/<product>/bin`, Toolbox 1.x channel layout, Toolbox scripts, `/opt/<product>`, snap, Flatpak export wrappers (system and user), `xdg-open` |

Flatpak needs no special launch plan: the export wrapper under `flatpak/exports/bin/<app-id>` is an executable that accepts the directory argument like the native launcher.

### 4. macOS bundles are launched through `open -a`

A discovered `.app` bundle is reported as the executable path. The launch plan becomes `/usr/bin/open -a <bundle> <dir>` so macOS applies its normal single-instance and activation behavior; a Toolbox script or a `PATH` binary is launched directly with the directory as its one argument. The directory is never interpolated into a shell string on any platform.

### 5. Version and edition come from `product-info.json`

JetBrains ships `product-info.json` at the product root on Windows and Linux and under `Contents/Resources` on macOS. Reading `version` and `productCode` from it (`IU`/`IC`/`WS`) fills the version and edition fields the UI already renders. A missing or unreadable file leaves both `null`; it never fails discovery.

### 6. Unix children join a new process group

`spawn_detached` calls `process_group(0)` on Unix so an IDE opened from VaneHub survives a terminal interrupt delivered to the app's own group. This mirrors the `DETACHED_PROCESS` flag already applied on Windows.

### 7. Embedded disclosure instead of nested cards

`SettingsDisclosure` gains an `embedded` flag that drops the outer frame and aligns its summary with `SettingsRow` padding, so opener management reads as one more row of the workspace panel. The advanced-configuration disclosure keeps its frame because it is a top-level section.

### 8. Opener commands leave the main thread

Tauri runs a synchronous command on the main thread. Discovery walks directories and, on Windows, spawns a dozen `reg`/`where` processes with three-second timeouts, so the three opener commands become `async` and hand the work to `spawn_blocking`. Launch no longer forces a rescan; it trusts the cached launcher as long as the path still exists and rescans only when it does not. Refresh emits the same event a preference save does, because the toolbar caches the catalog independently.

### 9. Directory fields share one component

Both directory settings had their own draft, blur handler, and error state, and both compared a display-normalized draft against the raw stored value. `DirectorySettingField` owns that once: display form on both sides of the comparison, Enter and blur commit, a two-second saved hint on a reserved line so the row never shifts, and a browse button backed by `settingsService.pickDirectory()`. The picker is a service-boundary operation so the component never touches the dialog plugin.

### 10. Capability flags are explicit

The startup toggle used the logging policy's "can open directory" flag as a stand-in for "is this the desktop runtime". The settings response now carries `launchOnStartupAvailable`, always true from native and false from the Web adapter, and the toggle reads that.

### 11. Reset attempts every key

Reset ran eleven saves in a row and stopped at the first failure, with the callbacks closing over the settings snapshot from before the reset started. The provider now reads the latest settings through a ref, and reset collects failures and reports them together after trying every key.

## Risks / Trade-offs

- Discovering a Linux program through `PATH` can find a distribution wrapper instead of a Toolbox launcher. The ranking prefers the JetBrains-managed script and app root over `PATH`, so the launcher Toolbox keeps current wins.
- `open -a` on macOS returns before the IDE finishes launching, which the spec already treats as completion.
