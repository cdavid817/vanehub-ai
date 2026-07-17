## Context

VaneHub currently has one Tauri webview window. Its active session and messages are persisted in SQLite, while advanced `ChatConfig` selections are owned by React state in the main window. The Rust runtime already emits global `chat:event` updates, but it does not expose a native auxiliary-window controller or typed events for active-session and settings/configuration changes.

The floating assistant must remain interactive when the main window is minimized or hidden, so an in-page fixed element is insufficient. The implementation must also preserve the existing React/service/native boundary, Web/mock usability, unified logging rules, two registered styles, and synchronized zh-CN/en resources.

## Goals / Non-Goals

**Goals:**

- Provide an opt-in, always-on-top Windows Tauri window with collapsed, quick-menu, and mini-chat modes.
- Keep the process and floating window available after the enabled main window receives a close request.
- Reuse the active VaneHub session and its exact persisted chat configuration across both windows.
- Synchronize session lifecycle, message streaming, configuration, language, and theme without sharing React memory between webviews.
- Preserve an interface-compatible Web/mock runtime and test the reusable surface in Chromium.
- Avoid new UI, state-management, or drag dependencies.

**Non-Goals:**

- A macOS or Linux native floating-window release in this change.
- A new assistant persona, autonomous suggestions, voice loop, mascot sprite, or proactive notification engine.
- Editing provider, model, permission, or reasoning controls inside the mini-chat window.
- Reimplementing the create-session form inside the floating window.
- Making the floating window a separate operating-system process.

## Decisions

### 1. Use a dedicated Tauri WebviewWindow controlled by Rust

The desktop adapter will request a Rust-native floating-window controller to create, show, resize, move, collapse, hide, restore the main window, or exit the application. The window uses a stable `floating-assistant` label and loads the existing Vite entry with a surface discriminator such as `index.html?surface=floating-assistant`.

The builder will create a transparent, undecorated, non-resizable, skip-taskbar, always-on-top window. Creation will occur from setup when the persisted setting is enabled or through an asynchronous command when a user enables it. This avoids the Windows deadlock risk of creating a WebviewWindow from a synchronous command or window event.

Alternative considered: an in-page `position: fixed` host like Clowder AI. Rejected because it disappears with the main window. A separate native process was also rejected because it would duplicate runtime ownership and complicate SQLite/process coordination.

### 2. Keep components behind a dedicated service boundary

A `FloatingAssistantService` will describe runtime availability, configuration, surface-mode changes, native dragging, main-window actions, application exit, and typed event subscriptions. `tauri-floating-assistant-client.ts` will be the only frontend module that invokes native commands; `web-floating-assistant-client.ts` will implement the same interface and report native-window operations as unavailable.

React components will not import Tauri APIs. The Web implementation will allow the floating surface to render as a normal test/preview document, while Basic Configuration clearly labels the real desktop function as unavailable outside Tauri.

Alternative considered: adding window methods to `AgentService`. Rejected because native window lifecycle is not an agent capability and would make the existing service less cohesive.

### 3. Render a separate React surface from the shared entry point

`main.tsx` will choose between the existing application and `FloatingAssistantApp` from the surface discriminator. The floating root will reuse i18n, settings/theme providers, query client behavior, semantic CSS tokens, and shared chat presentation components, but it will not mount the workspace router or notification shell.

The native window will use three logical modes:

- `collapsed`: compact Bot/Sparkles button and lifecycle status dot.
- `menu`: new session, return to active session, mini chat, settings, and exit actions.
- `chat`: active-session header, message history, streaming/thinking/tool/error rendering, input, send, stop, collapse, and return-to-main actions.

Window sizes are native logical sizes rather than large transparent click-blocking rectangles. Mode changes preserve a bottom-right anchor, recompute the top-left origin, and clamp the result to a monitor work area.

### 4. Persist a monitor-safe floating anchor, not mode-specific origins

The native configuration stores `enabled` plus an optional desktop anchor with coordinates and monitor identity. The anchor represents the stable lower-right point used by all surface sizes. At startup, monitor removal, DPI change, or mode resize, Rust resolves the saved monitor or falls back to the nearest/primary monitor and clamps the window into the available work area.

Native dragging is initiated through the service. The adapter observes move completion and sends a debounced persistence request so SQLite is not written for every move event. Invalid or off-screen persisted values fall back to the default lower-right position.

Alternative considered: persisting a React/CSS position. Rejected because CSS coordinates are local to one webview and do not safely survive native DPI and multi-monitor changes.

### 5. Define explicit close, restore, and exit semantics

When the main window receives `CloseRequested`, Rust checks whether the floating assistant is enabled and can be ensured visible. On success it prevents close and hides the main window. If the assistant cannot be created or shown, the close proceeds so the user is not stranded with a headless process; the failure is written through unified redacted logging.

Minimization needs no interception. `return-current-session`, `new-session`, and `open-settings` show, unminimize, and focus the main window, then emit a targeted main-window action. The main React surface handles the action through the service subscription and opens the existing route/dialog. `exit` uses the native application-exit path and is not translated into another close/hide request.

### 6. Persist session chat preferences and compose an effective configuration

A backward-compatible SQLite migration adds a nullable configuration snapshot for each session. The snapshot stores provider/model/permission/reasoning and streaming/thinking/long-context preferences. Stable session `agent_id` and `interaction_mode` remain authoritative and are composed into the returned `ChatConfig`; they are not duplicated as independently writable preferences.

`AgentService` gains typed get/save session-chat-configuration operations in both Tauri and Web adapters. The main chat hook loads the snapshot for the active session and saves validated changes. The mini chat only reads the effective configuration. Old sessions without a snapshot receive validated defaults derived from their stable agent and existing CLI parameter profile; the first explicit update persists a snapshot.

Deleting a session removes its configuration through the existing session lifecycle. Invalid or obsolete provider/model values are normalized to supported values before use rather than passed to a CLI.

Alternative considered: keep configuration in main-window React state and derive defaults independently in the mini chat. Rejected because the two windows could silently use different models or permission modes.

### 7. Synchronize through persisted truth and narrow typed events

SQLite/native state remains authoritative. Existing `chat:event` continues to carry stream updates. New narrow events notify both windows that active-session state, session configuration, or application settings changed; receivers invalidate/refetch the relevant query rather than treating event payloads as the durable record.

The Rust send path will reject a second generation for the same session before inserting duplicate user/assistant messages. Both windows derive busy state from persisted lifecycle/messages and global events, preventing simultaneous sends from creating competing processes.

### 8. Use semantic design tokens, localized labels, and accessible controls

The floating assistant uses Tailwind and shared semantic tokens only. `futuristic` provides the dark restrained translucent treatment through token values; `minimal` provides the bright solid low-shadow treatment through the same component classes. No component branches on a theme name and no inline style is introduced for visual styling.

Bot/Sparkles icons identify the surface. A semantic status dot maps no-session/idle/starting/running/failed/stopped states without being the only status signal; translated accessible text exposes the same state. Keyboard activation, focus visibility, Escape behavior, labels/tooltips, reduced motion, and compact-width clipping are covered by tests.

All native diagnostic failures use the unified logging service with redacted coordinates/context and do not create a feature-local log file.

## Risks / Trade-offs

- [Transparent always-on-top windows vary across Windows/WebView2 versions] → Keep the surface rectangular only at its current native size, avoid effects that require unsupported composition, and include Windows 10/11 smoke checks.
- [A failed floating-window creation could leave the main window hidden] → Ensure the window before preventing close; allow normal exit and record a unified diagnostic if ensure/show fails.
- [Two webviews can race on session/config state] → Keep SQLite authoritative, emit invalidation events after commits, and enforce single active generation in Rust before message insertion.
- [Multi-monitor coordinates become invalid after monitor/DPI changes] → Persist a monitor-aware anchor, clamp on every restore/resize, and fall back to the primary monitor.
- [A second WebView2 instance increases memory usage] → Create it only when enabled, keep one instance while enabled, and destroy it when disabled.
- [Playwright cannot prove native always-on-top or Windows close semantics] → Cover reusable UI and adapter contracts in automated tests, cover native decisions with Rust tests, and require a documented Windows smoke matrix.
- [Session configuration persistence changes previously ephemeral behavior] → Use a nullable additive migration, normalize old values, and retain derived defaults for sessions without a snapshot.

## Migration Plan

1. Add nullable session configuration storage and floating-assistant config storage with migration tests; existing sessions and users remain disabled/defaulted.
2. Add service contracts and Web/Tauri adapters without mounting the native surface.
3. Move the main chat hook to service-backed session configuration and verify existing chat behavior.
4. Add typed cross-window events and native concurrency enforcement.
5. Add the floating React surface and Web-renderable tests.
6. Add native window creation/lifecycle/position behavior and capability permissions.
7. Add the Basic Configuration toggle, localization, theme QA, and Windows smoke verification.

Rollback is safe because the migration is additive and nullable. Older binaries ignore the new setting/configuration data. Disabling the feature destroys the auxiliary window and restores ordinary main-window close behavior.

## Open Questions

No product decisions remain open for the first version. Exact logical dimensions and animation durations may be tuned during dual-theme visual QA without changing the behavioral contract.
