## Why

The Windows desktop client can intermittently become a white, unusable surface after users switch VaneHub pages and then switch away from and back to the application. React error boundaries and existing client logs do not observe WebView2 renderer-process failures, so the current application neither records the native failure nor recovers the affected main surface. Live testing also reproduced a second blank-surface path: an asynchronous frontend bootstrap rejection left `#root` empty until the WebView was manually reloaded.

## What Changes

- Observe Windows WebView2 process-failure events for the main desktop WebView and write redacted diagnostics through unified logging.
- Reload the main WebView after an unexpected main-frame renderer exit, and recover from persistent renderer unresponsiveness only after a bounded repeat threshold.
- Restart the desktop process if the shared WebView2 browser process exits unexpectedly and the existing WebView cannot be recovered in place.
- Leave auto-recoverable GPU, utility, and subframe failures to WebView2 while still recording diagnostics.
- Catch frontend surface-bootstrap failures before React mounts, render a visible localized recovery surface, and let the user retry by reloading the current entry point.
- Report frontend bootstrap failures through the existing service boundary and unified logging when available without allowing diagnostic failure to suppress recovery UI.
- Add native policy tests plus frontend regression coverage for switching among retained session pages and restoring the active terminal surface.
- Keep native process observation Windows-only while sharing the frontend bootstrap fallback with Web/mock mode without requiring a desktop API.

## Capabilities

### New Capabilities

- `desktop-webview-reliability`: Defines native WebView process-failure diagnostics, bounded recovery policy, recoverable frontend bootstrap behavior, and page/application-switch regression behavior.

### Modified Capabilities

None.

## Impact

- Windows-only Tauri bootstrap and WebView2 host integration plus the shared frontend entry point.
- Unified native diagnostic logging; no feature-local log files and no unredacted process details.
- A Windows-target dependency matching the WebView2 COM version used by the pinned Tauri runtime.
- Localized bootstrap-recovery copy and frontend regressions; no new frontend service API, direct `invoke()` usage, database migration, or Web/mock adapter change.
