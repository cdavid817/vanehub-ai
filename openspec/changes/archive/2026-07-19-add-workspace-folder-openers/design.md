## Context

The session workspace currently renders eight accessible tabs in one horizontally scrollable tab list. Session metadata already carries `worktreePath`, `folder`, `projectPath`, and remote-workspace information, while native workspace queries already resolve local roots in worktree-first order. Basic Settings persists scalar application settings through the desktop context and a Web adapter, and external processes are constructed centrally with explicit arguments in `platform::process`.

This change crosses the session toolbar, settings UI, frontend service adapters, desktop settings, workspace authorization, SQLite persistence, process launching, and unified logging. Windows installations are not reliably discoverable through `PATH` alone: Store aliases, custom VS Code/Git locations, and JetBrains Toolbox require a bounded multi-source detector. GUI and terminal programs must also be spawned without waiting for their lifetime to end.

## Goals / Non-Goals

**Goals:**

- Provide six stable opener identities: `vscode`, `file-explorer`, `windows-terminal`, `git-bash`, `intellij-idea`, and `webstorm`.
- Discover installed Windows programs through bounded, deterministic sources without launching them during availability checks.
- Persist one configured default plus an enabled multi-select atomically, retain unavailable selections, and compute a safe effective fallback.
- Open only the authorized effective root of the selected local session through an allowlisted detached process plan.
- Keep desktop and Web/mock adapters contract-compatible and keep Tauri invocation out of React components.
- Preserve accessible tab semantics and narrow-layout behavior while fixing the opener control at the right edge.

**Non-Goals:**

- Arbitrary executable paths, custom arguments, user-defined opener plugins, or whole-disk discovery.
- VS Code Insiders, Cursor, Visual Studio, PowerShell, CMD, WSL distributions, or additional JetBrains products in the first release.
- Selecting among multiple installations or separately modeling IntelliJ Community and Ultimate.
- Opening remote workspaces, forcing reuse of an existing third-party window, or confirming that an IDE finished loading a project.
- Managing the lifetime, output, or PID of the launched external program after the OS accepts the spawn.

## Decisions

### Stable catalog and availability are separate from preferences

The desktop domain owns a fixed catalog with display metadata, category, icon key, and launch strategy. Discovery returns status, resolved executable path, optional version/edition, detection source, and a safe reason. Preferences persist only stable ids; executable paths, versions, and detection status remain runtime-derived.

This prevents stale paths after upgrades or Toolbox version changes and keeps the persisted contract independent of installation layout. Persisting detected paths was rejected because paths are environment state, not user intent.

### Detection is bounded, lazy, cached, and revalidated before launch

The detector examines only system-known components, `PATH`/`where`, App Paths and uninstall registry records, product-specific known locations, the Git installation inferred from `git.exe`, and bounded JetBrains Toolbox locations. It never recursively scans drives and never invokes an interactive program for readiness.

The first toolbar/settings request starts an asynchronous detection and caches its result for the application session. Manual refresh bypasses the cache. Before launch, the selected executable and target directory are checked again so an uninstall between detection and click becomes a concise unavailable failure rather than a stale spawn attempt. Per-opener failures produce partial results instead of failing the catalog.

Using only `PATH` was rejected because custom GUI installs commonly omit it. Broad filesystem search was rejected for latency, privacy, and project rules governing long-running scans.

### Git Bash has product-specific identity

Git Bash is detected from `git-bash.exe` or a validated Git for Windows installation, not from `bash.exe`. This prevents Windows/WSL Bash from being misclassified. The launch plan uses the target as the child working directory and only adds a directory flag if compatibility tests establish a stable Git for Windows contract.

### Preferences are saved as one validated transaction

The preference aggregate contains `configuredDefaultOpenerId` and ordered, de-duplicated `enabledOpenerIds`. File Explorer is always enabled as the Windows fallback. A newly selected default must be enabled and currently available. Unavailable non-default selections remain stored so they become usable after reinstall.

The repository stores stable values under dedicated settings keys but exposes a dedicated atomic preference mutation instead of two generic scalar saves. This prevents a crash or failure from persisting an enabled list that excludes the saved default. Existing installations receive defaults without a destructive schema migration: VS Code is configured when detected on first resolution, otherwise File Explorer is configured; File Explorer remains the deterministic persisted fallback when environment-dependent initialization cannot be performed safely.

### Configured and effective defaults remain distinct

If the configured default is enabled and available, it is effective. Otherwise File Explorer is effective. If platform initialization cannot supply File Explorer, the first enabled available opener is used; if none exists, the action is unavailable. Fallback does not overwrite the configured value, so reinstalling the preferred program restores the intended behavior. DTOs expose both ids and a `fallbackActive` flag so the UI never silently changes meaning.

### Workspaces authorizes the target; desktop launches it

The frontend sends only `sessionId` and `openerId`. The workspaces application resolves the session, rejects remote sessions, and selects `worktreePath`, then `folder`, then `projectPath`. It verifies that the result is an existing directory before calling a folder-launch port. A workspaces infrastructure gateway delegates the trusted local path and stable opener id to the published desktop opener API.

Desktop owns discovery, preference, and launch-plan rules; workspaces owns session and path authorization. Passing arbitrary paths or executable strings through a Tauri command was rejected because it would turn the UI boundary into a generic process launcher.

### External applications use a detached explicit-argument process primitive

`platform::process` gains a detached spawn operation that accepts an executable, an argument vector, and a working directory, applies existing executable validation, and does not invoke `cmd`, PowerShell, or another shell. Desktop infrastructure maps each catalog id to a fixed plan, for example Explorer with one directory argument, Windows Terminal with `-d` plus the directory, and JetBrains/VS Code with one directory argument.

Spawn acceptance is the terminal success condition. VaneHub does not wait for the external process, capture its output, or own its lifecycle. Reusing the existing bounded output executor was rejected because it would hold pipes and operations open for the lifetime of an IDE.

### One frontend contract, runtime-specific adapters

The frontend service boundary exposes list/refresh availability, get/save preferences, and open-session-folder operations. The Tauri adapter alone invokes the corresponding commands. The Web/mock adapter returns deterministic catalog and preference data, persists mock preferences, and returns an explicit unavailable result for native launch. React components consume the service interface and subscribe to preference-change events rather than importing Tauri APIs.

### The opener is a sibling of the tab list

The toolbar wraps the existing `role="tablist"` and a separate split button. Only the tab list scrolls horizontally; the opener remains fixed. The main action shows the effective default icon/name and the menu lists enabled available openers, current selection, unavailable configured-default feedback, and a navigation action to Basic Settings. Keyboard navigation within the eight tabs remains unchanged and the menu uses standard button/menu focus behavior.

### Diagnostics use unified logging without raw target paths

Discovery and launch diagnostics use the unified logging service with opener id, detection source, safe status/error code, session id when applicable, and target kind (`worktree`, `folder`, or `project`). Normal records do not include the full executable or workspace path. Detailed native errors are redacted before persistence, while the UI receives concise localized failures.

## Risks / Trade-offs

- [Windows registry and Store alias layouts vary] -> Treat each source as optional, return partial per-opener status, cover detectors with fake ports, and revalidate before launch.
- [Multiple IDE installations make selection surprising] -> Use deterministic product-specific ranking and show the resolved edition/version/path; defer user installation selection to a later change.
- [File Explorer forced enablement reduces user choice] -> Keep it as a visible locked fallback so the toolbar retains a safe Windows action after other tools are uninstalled.
- [Detached spawn success does not prove the folder was rendered] -> Define success as OS process acceptance and avoid promises about third-party application state.
- [Git Bash working-directory behavior differs by release] -> Add a compatibility-focused implementation task and tests before fixing optional arguments.
- [Toolbar width can regress existing tab access] -> Keep the action outside the tab list, preserve internal tab scrolling, and add narrow-viewport component/E2E coverage.
- [Brand icons carry licensing and theme concerns] -> Use reviewed bundled assets or approved neutral icon mappings; never load arbitrary executable resources in React.

## Migration Plan

1. Add recognized preference keys/defaults and transactional repository support without changing existing rows.
2. Add catalog, detector, launch planning, workspaces gateway, Tauri commands, and detached process support behind tests.
3. Extend frontend contracts and both runtime adapters before rendering new controls.
4. Add Settings and session-toolbar UI, localization, and event synchronization.
5. Validate existing settings with missing keys fall back safely; rollback removes the UI/commands while leaving unknown key-value rows harmless.

## Open Questions

- Confirm the reviewed icon source before implementation; neutral Lucide-style mappings remain the fallback if branded assets are not approved.
- Confirm Git for Windows versions supported by the `git-bash.exe` working-directory launch plan during the implementation spike.
