## Context

The reported failure occurs in the Windows Tauri client after internal page switching followed by leaving and restoring the application. Unified logs show normal React/native activity and no error-boundary event, while repeated testing confirms that ordinary route and retained-tab switching does not deterministically blank the DOM. This identifies one failure layer below React, at the WebView2 process/compositor boundary. Microsoft documents `ProcessFailed` as the host signal for unexpected browser, renderer, GPU, and utility-process failures and requires host recovery for browser-process and main-frame renderer failures.

Live testing of the first implementation exposed a separate pre-React failure layer. The Tauri WebView initially loaded with an empty `#root`, no JavaScript console error, and no rendered recovery control; a manual reload immediately restored the workspace. The frontend entry point starts an asynchronous dynamic import without handling its rejection, so a transient module/bootstrap failure can leave the document permanently blank before an error boundary exists.

The Tauri main window currently relies on WebView2 defaults and does not subscribe to this event. Native logging already has a unified adapter, and agent terminal processes are retained independently of the React renderer, so a WebView reload can restore the UI and reattach the active terminal without terminating the underlying agent.

## Goals / Non-Goals

**Goals:**

- Prevent an unexpected main-frame renderer failure from leaving the desktop client as a permanent white surface.
- Record process-failure kind and selected recovery action through the existing redacted unified logging path.
- Bound recovery from renderer unresponsiveness so a transient busy period does not immediately reload the UI.
- Preserve agent/session data by relying on existing persisted state and retained terminal reattachment after reload or restart.
- Keep the recovery policy independently testable without requiring a live WebView2 crash in CI.
- Ensure every frontend surface-bootstrap rejection produces a visible, localized retry surface instead of an empty root.
- Report bootstrap failures through the frontend service boundary when available without making diagnostics a prerequisite for recovery.

**Non-Goals:**

- Disabling GPU acceleration globally or pinning an Evergreen WebView2 runtime.
- Reloading on every focus, visibility, resize, GPU-process exit, or utility-process exit.
- Changing React routes, settings-page retention, service interfaces, SQLite schema, or normal browser/Web behavior beyond the shared bootstrap fallback.
- Claiming a deterministic reproduction of an operating-system/WebView2 process failure in Playwright.

## Decisions

### Register a Windows-only WebView2 process-failure observer from Tauri setup

The bootstrap will pass the main WebView to a small native reliability module through Tauri's `with_webview` hook. On Windows, the module obtains `ICoreWebView2`, registers a `ProcessFailedEventHandler`, and keeps all COM-specific code behind `cfg(windows)`. The direct `webview2-com` version will be pinned to the version resolved by the pinned Tauri runtime because Tauri documents platform WebView handles as a version-sensitive integration.

Alternative considered: a React `focus` or `visibilitychange` handler cannot execute when the renderer has exited or is unresponsive and cannot observe browser/GPU process failures.

### Use failure-kind-specific recovery

- `RenderProcessExited`: reload the main WebView immediately. If reload fails, restart the application.
- `RenderProcessUnresponsive`: record the first signal and reload only on the second signal within 45 seconds. A later isolated signal starts a new window. If reload fails, restart.
- `BrowserProcessExited`: restart the application because the existing WebView is closed and cannot reload.
- GPU, utility, sandbox helper, plugin, subframe, and unknown exits: record only; WebView2 automatically recovers the applicable process or limits impact to a subframe.

Alternative considered: immediate reload for every failure is simpler but would turn harmless, auto-recoverable GPU exits into avoidable application disruption. Disabling hardware acceleration hides symptoms, increases rendering cost, and removes useful failure classification.

### Keep policy logic separate from COM registration

A platform-neutral state machine maps normalized failure kinds and timestamps to `Observe`, `Reload`, or `Restart`. Unit tests cover all kinds, the unresponsive threshold, threshold expiry, and reset after recovery. The Windows adapter is intentionally thin and translates WebView2 enums into that policy.

Alternative considered: testing only source strings or a live WebView2 crash would either miss behavioral errors or be too flaky for CI.

### Preserve the existing frontend runtime boundary

No React component calls Tauri APIs. Web/mock mode requires no desktop API and gains the same bootstrap fallback. Frontend regression coverage verifies that retained workspace panels keep the active agent terminal mounted while users switch pages, allowing a restored or reloaded desktop surface to reuse the existing session/terminal recovery path.

### Guard asynchronous frontend bootstrap before React mounts

The frontend entry point will handle the promise returned by surface selection and dynamic import. On rejection it will synchronously replace the root contents with a small localized recovery surface whose implementation is bundled with the entry point rather than the dynamically imported application surface. The retry action reloads the current entry point, which recovered the reproduced failure and preserves whether the user opened the main or floating surface.

After the recovery surface is visible, the handler will make a best-effort diagnostic call through the existing settings-service boundary. Failure to load or invoke that reporting path is swallowed because logging must not remove the only recovery control. The diagnostic contains the surface type and normalized error text; the native unified logger remains responsible for redaction before persistence.

Alternative considered: a React error boundary cannot catch a rejected dynamic import that occurs before the React root mounts. Retrying only the failed import would also leave partially initialized entry-point state harder to reason about than a full reload.

## Risks / Trade-offs

- [Reload loses volatile React component state] → Reload only for main-frame failure or repeated unresponsiveness; persisted sessions, configuration, and retained terminals restore through existing boundaries.
- [A false-positive unresponsive event could reload during temporary system pressure] → Require two events within 45 seconds, matching WebView2's repeated unresponsive signaling model.
- [COM integration can drift when Tauri updates] → Pin the matching `webview2-com` version on Windows and cover compilation in Rust checks.
- [Restart could loop after repeated browser-process crashes] → Browser-process failure is exceptional; log every restart-triggering failure so recurrence is diagnosable. A future crash-loop guard can be added if field evidence shows repetition.
- [A GPU-only white flash can still occur] → WebView2 documents GPU exit as auto-recoverable; record it but avoid disruptive reload unless evidence shows persistent failure.
- [The recovery UI could depend on the same failed application chunk] → Keep the fallback renderer and copy lookup in the statically loaded entry bundle and avoid importing the application surface to display it.
- [Diagnostic reporting can fail during bootstrap] → Render recovery UI first and treat service-boundary reporting as best effort.

## Migration Plan

1. Add and strictly validate the OpenSpec artifacts.
2. Add the policy module, Windows WebView2 observer, and unified diagnostic events.
3. Register the observer for the main WebView during setup and add native/frontend regressions.
4. Guard the asynchronous frontend bootstrap with a visible retry surface and service-boundary diagnostics.
5. Run frontend, Rust, and strict OpenSpec validation, then reproduce repeated page/minimize/restore switching in a dev WebView.

Rollback removes the observer registration and Windows-only dependency; there is no database or persisted-data migration.

## Open Questions

None. If diagnostics later show only repeated GPU failures, evaluate a targeted GPU mitigation in a separate evidence-backed change.
