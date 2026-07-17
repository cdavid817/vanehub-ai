## Why

VaneHub AI currently has a static notification affordance but no shared mechanism for components to publish, present, or manage user-facing notifications. A unified in-app framework is needed now so feature teams can report operation outcomes consistently without coupling components to a specific toast implementation or runtime.

## What Changes

- Add an application-wide, React Context-based notification API for publishing, dismissing, reading, and clearing notifications.
- Add transient toast presentation and a top-bar notification center with unread state and recent notification history.
- Support success, error, warning, and informational notifications with global or session scope.
- Localize framework-owned text in Simplified Chinese and English and adapt presentation to both futuristic and minimal themes.
- Document the first-version in-memory boundaries and the planned evolution toward persistence, runtime events, richer actions, and operating-system notifications.
- Add focused state/UI tests and Playwright coverage for the main notification workflow.

## Capabilities

### New Capabilities
- `notification-system`: Defines the shared notification model, publishing API, lifecycle, scope, presentation, localization, accessibility, and first-version persistence boundary.

### Modified Capabilities
- `main-layout-ui`: Makes the existing top-bar notification control interactive and exposes unread/recent notification state without disrupting the workspace layout.

## Impact

- Affects both Tauri desktop and Web runtime through a runtime-neutral frontend context; no Tauri command, database schema, service-adapter method, or new dependency is introduced in the first version.
- Adds notification context/hooks and presentational components under `src/`, integrates the provider at the application root, and connects the top bar to the notification center.
- Extends `zh-CN` and `en` locale resources and frontend test coverage.
- Preserves the frontend/native boundary: components do not call `invoke()` and persistent or OS-level delivery remains outside this change.
