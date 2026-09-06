## 1. Native discovery

- [x] 1.1 Extract discovery into `folder_opener_discovery.rs` with an explicit `DiscoveryEnv` and per-platform candidate builders.
- [x] 1.2 Implement Linux candidates: `PATH`, Toolbox scripts and app roots, legacy Toolbox channels, `/opt`, snap, and `xdg-open`.
- [x] 1.3 Implement macOS candidates: application bundles in `/Applications` and `~/Applications`, Toolbox scripts, `PATH`, and `/usr/bin/open`.
- [x] 1.4 Add the Toolbox 2.x `%LOCALAPPDATA%\Programs` root and `%ProgramFiles(x86)%\JetBrains` to Windows JetBrains discovery.
- [x] 1.5 Read `product-info.json` for version and edition beside a discovered JetBrains launcher.
- [x] 1.6 Return `not-installed` for a supported-but-absent program and `unsupported-platform` only for Windows Terminal and Git Bash off Windows.
- [x] 1.7 Build the launch plan per platform and detach Unix children into their own process group.
- [x] 1.8 Add unit tests covering status semantics, Linux and macOS candidate ranking against a temporary tree, `product-info.json` parsing, and the macOS bundle launch plan.

## 2. Settings presentation

- [x] 2.1 Add an `embedded` variant to `SettingsDisclosure` and use it for opener management.
- [x] 2.2 Show a localized disabled empty-state option in the default-opener select when no enabled opener is available.
- [x] 2.3 Present availability with `StatusPill` and align the reorder controls with the opener identity row.
- [x] 2.4 Use the shared `PageHeader` on Basic Configuration.
- [x] 2.5 Add the empty-state key and a platform-neutral default-folder placeholder to every registered locale.
- [x] 2.6 Add component tests for the empty state and the status pill.

## 3. Responsiveness and page ergonomics

- [x] 3.1 Make `list_folder_openers`, `refresh_folder_openers`, and `open_session_folder` async on a blocking worker; emit the opener event after a refresh; reuse the cached launcher at launch and rescan only when it is gone.
- [x] 3.2 Validate that a non-empty default folder path is an existing directory at the native settings boundary.
- [x] 3.3 Add `launchOnStartupAvailable` to the settings response and gate the startup toggle on it.
- [x] 3.4 Add `pickDirectory` to the settings service (native dialog; Web rejects) and a shared `DirectorySettingField` with browse, Enter-to-commit, saved hint, and normalized comparison; use it for the default folder and log directory.
- [x] 3.5 Make reset attempt every key and aggregate failures; read the latest settings through a ref so sequential saves never build on a stale snapshot.
- [x] 3.6 Localize font-size labels, derive theme labels from the registry, remove developer storage notes, merge the three info tile components, add Node.js re-detect, keep reorder buttons outside the opener checkbox label, and let proxy Clear reset the bypass list.
- [x] 3.7 Add tests for the directory field, reset continuation, and the opener card, and update the settings DTO contract test.

## 4. Verification

- [x] 4.1 Run the full validation command list from `AGENTS.md`.
- [x] 4.2 Confirm on this Linux host that IntelliJ IDEA Ultimate, WebStorm, VS Code, and the file manager are reported available with their resolved paths.
