## Context

Basic Configuration currently renders each capability as a peer card, even though language, appearance, startup, and workspace defaults are changed far more often than proxy, logging, storage, or runtime diagnostics. The shared settings model already contains `defaultFolderPath`, but the page does not expose it. Existing components correctly use settings and feature service boundaries, so the change can remain frontend-only.

## Goals / Non-Goals

**Goals:**

- Make the first viewport contain the settings most users understand and change frequently.
- Organize controls by user intent rather than by implementation service.
- Preserve every existing setting and native/Web capability state through progressive disclosure.
- Add the missing default project directory control through `SettingsProvider`.
- Keep shared row/group styling responsive and accessible.

**Non-Goals:**

- Adding new settings navigation destinations or moving capabilities to new routes.
- Adding a native folder-picker command or changing the settings service contract.
- Changing persistence, SQLite, folder-opener discovery, proxy, logging, or floating-assistant runtime behavior.
- Redesigning settings pages other than Basic Configuration.

## Decisions

### Use four intent-based groups

Basic Configuration will render common preferences, startup and window behavior, workspace defaults, and advanced configuration in that order. This keeps the implementation within the existing page and avoids a broad settings-navigation migration. The alternative of immediately creating separate Network, Data, and Diagnostics pages would produce cleaner long-term ownership but requires wider navigation, search, and specification changes.

### Collapse operational content by default

Network proxy, logs, data management, and Node.js runtime information will remain mounted inside one native `details` disclosure. Native disclosure preserves keyboard support and keeps every existing component and service call intact. The trade-off is that background loading in mounted advanced components still occurs; deferred mounting is outside this change.

### Compose existing feature controls as rows

Startup, floating-assistant, and folder-opener components will render embeddable row content rather than separate top-level cards. Basic Configuration owns the group hierarchy, while feature components retain their existing service calls, error reporting, and runtime availability logic.

### Save the default project directory through SettingsProvider

The page will edit `settings.defaultFolderPath` and call `saveSetting("defaultFolderPath", value)` on blur. This uses the existing React service boundary and works in both Tauri and Web/mock runtimes. A native directory picker is not introduced because no such shared settings capability currently exists.

### Make reset a deliberate footer action

The reset action will move below all groups and require localized confirmation. This reduces accidental activation and communicates that reset is a broad maintenance action rather than a primary page action.

## Risks / Trade-offs

- [Advanced controls are less immediately visible] → Keep a clear localized disclosure summary that names the included categories and remains searchable in the DOM.
- [Default project directory remains a text field] → Explain the expected path and preserve a future path-picker enhancement behind the service boundary.
- [Refactoring feature sections could alter loading or error presentation] → Retain feature-owned hooks and service calls and add focused rendering tests for grouping, labels, disclosure, and reset behavior.
- [The current settings search does not filter Basic Configuration] → Do not claim search support in this change; address settings search semantics separately.
