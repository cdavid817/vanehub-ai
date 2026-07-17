## 1. Notification State Foundation

- [x] 1.1 Define typed notification records, scopes, reducer actions, queue bounds, and id creation without runtime-specific dependencies
- [x] 1.2 Implement `NotificationProvider` and `useNotifications()` with publish, read, remove, clear, and toast-lifecycle operations
- [x] 1.3 Add focused reducer/provider tests for lifecycle, unread counts, scope retention, and bounded queues

## 2. Notification Presentation

- [x] 2.1 Implement the scoped, auto-dismissing toast viewport with semantic Lucide icons and accessible controls
- [x] 2.2 Implement the top-bar notification center popover, unread indicator, empty state, read actions, outside-click, and Escape handling
- [x] 2.3 Integrate the provider and viewport with the application shell and publish notifications from a real session workflow

## 3. Themes and Localization

- [x] 3.1 Add parity-checked Simplified Chinese and English framework strings and localized session-workflow notification copy
- [x] 3.2 Verify toast and notification-center layout against futuristic/minimal tokens and narrow viewport constraints

## 4. Verification and Documentation

- [x] 4.1 Add component/Playwright coverage for publishing, auto-dismiss/history retention, unread management, popover dismissal, theme, locale, and narrow viewport behavior
- [x] 4.2 Run `npm run test` and `npm run build`; attempt `npm run lint` and record that the repository has no lint script
- [x] 4.3 Run `cargo test`, `cargo check`, and `cargo clippy` against `src-tauri/Cargo.toml`
- [x] 4.4 Run strict OpenSpec validation and reconcile implementation, specs, and documented optimization roadmap

## 5. Post-Verification Refinements

- [x] 5.1 Separate public notification actions from private toast presentation lifecycle operations
- [x] 5.2 Add Playwright coverage proving in-memory notifications reset after a page reload
- [x] 5.3 Add the missing npm lint gate and align stale main-chat and Skills Playwright workflows with the current UI and locale behavior
- [x] 5.4 Re-run frontend unit/build/lint, full Playwright, Rust, and strict OpenSpec validation
