## Context

`bootstrap::runtime::run` builds the Tauri application with dialog, opener, updater, process, and autostart plugins, and nothing in that chain arbitrates between processes. Each operating-system launch therefore reaches `setup`, which opens `NativeDatabase` against the application data directory, installs a process-wide log-receipt sink, assembles every bounded-context API, and starts the background jobs. A second launch repeats all of it against the same profile.

## Goals / Non-Goals

**Goals**

- One VaneHub process per desktop session, with the running instance surfaced when a duplicate launch is requested.
- Correct behavior when the running instance is hidden in the tray, which is the common case given close-to-tray.
- No regression for helper subprocesses that re-execute the same binary.

**Non-Goals**

- Arbitrating between different application data profiles. The guard is per desktop session, not per profile.
- Changing the tray restore path, close-to-tray behavior, or graceful quit.
- Forwarding launch arguments to the running instance. The desktop entry point takes no user-facing arguments.

## Decisions

### Use `tauri-plugin-single-instance` rather than a hand-rolled lock

The hard part of a single-instance guard is not detecting the duplicate but telling the running process to surface itself. The plugin already carries that inter-process channel per platform. A lock file alone would let the second process exit silently, which reads to the user as "I clicked and nothing happened" — worse than the bug being fixed, because close-to-tray means there is often no visible window to notice.

The plugin's callback signature is `FnMut(&AppHandle, Vec<String>, String)`, receiving the duplicate launch's argv and working directory. Neither is needed here; the callback only restores the window.

### Register the guard first in the builder chain

The guard must short-circuit before the other plugins initialize, so a duplicate launch does not partially construct a runtime it is about to abandon.

### Restore with show, unminimize, and focus

`show` alone is insufficient for a minimized window and `unminimize` alone is insufficient for a tray-hidden one, and a duplicate launch can encounter either. The three-step sequence already exists in `tauri_floating_assistant_window.rs` for the same "bring the main window back" intent, and is reused rather than reinvented.

The tray runtime's own `show_main_window` currently performs only show and focus. That path is left untouched: it is reached from a tray action rather than a duplicate launch, and widening it is a separate concern from this fix.

### Helper subprocesses stay outside the guard

The MCP relay re-executes this same binary with a relay flag. `lib.rs::run` dispatches that mode and returns before `bootstrap::run` is called, so a relay child never reaches the builder and never contends for the instance lock. This ordering is load-bearing: moving the guard into `main` or ahead of the relay dispatch would make the primary instance treat its own child as a duplicate launch and steal focus every time a relay starts. A wiring assertion pins the ordering.

### Only release builds arbitrate instances

On Windows the plugin's lock is a named mutex derived only from the bundle identifier — `format!("{id}-sim")` in the plugin's `platform_impl/windows.rs`, where `id` is `app.config().identifier`. It does not observe `VANEHUB_APP_DATA_DIR`.

Every build of this application shares `ai.vanehub.app`: `tauri.sidecar.conf.json` overrides only `bundle.externalBin`. That makes two non-production builds collide with the installed application and with each other:

- The desktop test client isolates state through `VANEHUB_APP_DATA_DIR` but keeps the production identifier, so a developer running desktop layers while an installed VaneHub is open would have the test client exit on startup and fail the run with a WebDriver connection error naming nothing about instance locking.
- `npm run tauri:dev` has the same identifier. A developer with the installed application in the tray would watch the dev build raise the *installed* window and then vanish, exiting inside plugin setup before any logging adapter exists to record why. Two worktrees running dev builds in parallel collide the same way, and this repository's workflow does that.

Keying the exemption on `debug_assertions` covers both under one rule — the test client is built with `tauri build --debug` — rather than exempting the test client and leaving the dev build to fail silently.

The cost is stated plainly: the guard is exercised only by a release build. It is verified by native unit tests, bootstrap wiring assertions, and manual launch verification on a packaged client.

### Restart requests exit rather than bypassing it

`AppHandle::restart()` spawns the successor and exits without emitting `RunEvent::Exit`. The plugin releases its mutex only on that event (`platform_impl/windows.rs`), so the successor can reach `CreateMutexW` while the dying process still holds the lock, treat itself as a duplicate, and exit — leaving no VaneHub running at all.

The update path therefore uses `request_restart()`, which requests exit and restarts from the normal exit path. That also restores something `restart()` was already skipping: this application's own `RunEvent::Exit` handlers, which drain the evidence bridge, shut down retained shells, and flush the log index worker.

The four `restart()` calls in `webview_recovery.rs` are left alone. They fire on WebView2 process failure, where `restart()`'s divergent return type is load-bearing for control flow, and restructuring crash-recovery paths without a harness that exercises them would trade a narrow timing race for a broader one.

## Risks / Trade-offs

- **No desktop end-to-end coverage.** Accepted for the reason above. A layer that exercised the guard would have to launch a second real process and assert the first took focus, which the current harness has no affordance for.
- **Plugin platform coverage.** The plugin is unavailable on Android and iOS by its own `cfg`. VaneHub ships desktop targets only, so this does not apply.
- **A narrow simultaneous-start race remains.** The plugin's Windows path creates its mutex and then its event-target window as two steps. A second process that observes `ERROR_ALREADY_EXISTS` but cannot yet find the window falls through and keeps running, because it has no one to hand the launch to. The exposed interval is the few instructions between those two calls in the first process, which a second process can only enter if both were started close enough to progress in lockstep; ordinary repeated clicking cannot reach it, since process startup is orders of magnitude longer. This is upstream behavior and is recorded rather than worked around.

## Migration Plan

No data migration. The change is additive at startup and takes effect on the next launch.

## Open Questions

None.
