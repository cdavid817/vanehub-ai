## Why

VaneHub currently loses its lightweight entry point when the main window is minimized or closed, forcing users to reopen the full workspace before they can resume a session or send a quick message. A Windows desktop floating assistant provides a persistent, low-friction path to active AI coding sessions while preserving the full workspace for complex operations.

## What Changes

- Add an opt-in Windows desktop floating assistant window that remains visible when the VaneHub main window is minimized or hidden.
- Provide collapsed ball, quick-menu, and mini-chat surfaces using VaneHub Bot/Sparkles visual language and lifecycle status indicators.
- Let users create a new session through the existing main-window dialog, return to the active session, open settings, chat with the active session, stop generation, and explicitly exit VaneHub.
- Change the enabled main-window close action to hide the main window while the floating assistant keeps the process alive.
- Persist floating-assistant enablement and screen position, with safe monitor/DPI clamping.
- Persist effective chat configuration per session so the main window and mini-chat window use the same agent, provider, model, permission, reasoning, streaming, thinking, and context selections.
- Synchronize settings, active-session state, configuration, message streaming, and lifecycle status across the two desktop windows.
- Keep the browser/Web runtime usable through interface-compatible adapters while reporting native floating-window operations as desktop-only.
- Localize all new user-visible text in Simplified Chinese and English and support both registered visual styles without theme-name branches.

## Capabilities

### New Capabilities

- `desktop-floating-assistant`: Windows native floating-window lifecycle, quick actions, mini chat, position persistence, cross-window synchronization, accessibility, and desktop/Web runtime behavior.
- `session-chat-configuration`: Session-level chat configuration persistence and consistent use across main and auxiliary chat surfaces.

### Modified Capabilities

- `settings-center-ui`: Add the localized floating-assistant setting and desktop-runtime availability behavior to Basic Configuration.
- `visual-design-system`: Extend token-first, dual-style visual requirements and visual QA to auxiliary desktop surfaces.

## Impact

- Frontend: new floating-assistant React surface, shared chat-state helpers, settings UI, i18n resources, and service contracts/adapters.
- Desktop runtime: Tauri multi-window creation and lifecycle handling, native positioning, main-window restore/hide/exit commands, and cross-window events.
- Storage: backward-compatible SQLite migration for floating-assistant settings/position and session chat configuration snapshots.
- Runtime boundaries: React components continue to call service interfaces; native window operations remain in the Tauri adapter/Rust layer, with an interface-compatible Web adapter.
- Testing: unit/component tests, Rust migration and lifecycle tests, Playwright coverage for the Web-renderable surface, and Windows native smoke verification for always-on-top, close/minimize, focus, DPI, monitor, and streaming behavior.
