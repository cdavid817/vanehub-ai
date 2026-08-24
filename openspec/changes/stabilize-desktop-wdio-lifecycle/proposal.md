## Why

The full WebdriverIO desktop run on `origin/main` failed five smoke workers even though all dedicated interaction layers passed. Four workers raced the embedded WebDriver shutdown between specs, while the remaining screen-sweep failure lacked the underlying frontend error detail needed to distinguish a product regression from test instrumentation.

## What Changes

- Stabilize the embedded WebDriver lifecycle before each desktop-test worker opens a session, without weakening owned-process shutdown guarantees.
- Preserve the browser error or rejection detail that causes desktop fatal-error instrumentation to trip, and include it in run-scoped evidence.
- Distinguish expected blocked cases from assertion failures in desktop WDIO evidence so release triage is actionable.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `desktop-runtime-verification`: Require stable worker-to-worker driver availability and diagnosable frontend-failure evidence for native desktop verification.

## Impact

- Affects the desktop-test-only WDIO harness and its evidence collection; the Web runtime and production Tauri artifact are unaffected.
- Does not change frontend/backend isolation or runtime adapter boundaries.
- May require a controlled patch or upgrade of the `@wdio/tauri-service` test dependency if its cross-platform lifecycle hook cannot be configured locally.
