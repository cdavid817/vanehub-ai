## 1. Persisted Configuration Foundation

- [x] 1.1 Add an additive SQLite migration for floating-assistant configuration and nullable per-session chat preference snapshots, including cascade/cleanup behavior.
- [x] 1.2 Implement Rust models, validation, normalization, default derivation, and load/save helpers for session chat configuration while keeping session agent id and interaction mode authoritative.
- [x] 1.3 Implement Rust load/save helpers for floating-assistant enablement and monitor-aware anchor data with disabled and invalid-position defaults.
- [x] 1.4 Add Rust migration and persistence tests for new users, existing sessions, invalid snapshots, per-session isolation, and deletion cleanup.

## 2. Frontend Service Contracts and Adapters

- [x] 2.1 Extend `AgentService` with typed get/save session-chat-configuration operations and implement matching Tauri and Web/mock adapters.
- [x] 2.2 Add the `FloatingAssistantService` contract, runtime adapter, Tauri client, and interface-compatible Web/mock client without importing Tauri APIs from React components.
- [x] 2.3 Add deterministic Web/mock storage and runtime-availability behavior for floating configuration, actions, events, and browser-renderable surface tests.
- [x] 2.4 Add contract and adapter tests for configuration round trips, normalization errors, desktop-only capability reporting, and event cleanup.

## 3. Shared Chat and Cross-Window State

- [x] 3.1 Refactor the main chat configuration hook to load and persist session preferences through `AgentService` while preserving existing selector behavior and derived defaults.
- [x] 3.2 Extract reusable message-event reduction and active-session chat-query behavior for use by both the workspace and mini-chat surfaces.
- [x] 3.3 Emit narrow committed events for active-session, session-configuration, floating-configuration, language, and theme changes, and invalidate/refetch authoritative data in both windows.
- [x] 3.4 Enforce one active generation per session in Rust before message insertion and add regression tests for simultaneous sends from separate surfaces.

## 4. Native Windows Floating Controller

- [x] 4.1 Add a dedicated Rust floating-assistant window module and asynchronous commands for ensure/show, mode resize, native drag, anchor persistence, main-window actions, disable/destroy, and explicit exit.
- [x] 4.2 Configure the `floating-assistant` WebviewWindow as transparent, undecorated, always-on-top, skip-taskbar, and surface-discriminated, and add its Tauri capability scope.
- [x] 4.3 Implement collapsed/menu/chat logical sizes, bottom-right anchor preservation, current-monitor work-area clamping, DPI/monitor fallback, and debounced position persistence.
- [x] 4.4 Intercept enabled main-window close requests to ensure the floating window and hide main, preserve ordinary minimize behavior, and allow normal exit when ensure/show fails.
- [x] 4.5 Route new-session, return-current-session, open-settings, and exit actions to explicit native behavior and targeted main-window events.
- [x] 4.6 Send native window failures through unified redacted logging and add Rust tests for close decisions, action routing, geometry helpers, and failure fallbacks.

## 5. Floating Assistant React Surface

- [x] 5.1 Split the shared Vite entry by surface discriminator and mount a lightweight `FloatingAssistantApp` provider stack without the workspace router/layout.
- [x] 5.2 Build the keyboard-accessible collapsed Bot/Sparkles control with translated lifecycle text, semantic status dot, drag affordance, focus treatment, and reduced-motion behavior.
- [x] 5.3 Build the translated quick menu with New Session, Return to Current Session, Mini Chat, Settings, collapse, and Exit VaneHub actions.
- [x] 5.4 Build the active-session mini-chat header, empty state, message history, thinking/tool/error rendering, input, send, stop, collapse, and return-to-main behavior.
- [x] 5.5 Synchronize mini-chat queries with active-session/configuration/message events and disable sending while the session already has an active generation.
- [x] 5.6 Keep new React component files within the project size limit and cover collapsed, menu, chat, empty, streaming, error, keyboard, and cleanup behavior with component tests.

## 6. Settings, Themes, and Localization

- [x] 6.1 Add a Basic Configuration section for the default-off floating assistant, immediate desktop enable/disable, close-to-hide explanation, saving/error state, and Web-runtime unavailable state.
- [x] 6.2 Add matching zh-CN and en resources for settings, quick actions, mini chat, lifecycle/status, errors, tooltips, accessibility, and native-runtime limitation text.
- [x] 6.3 Style every floating mode with shared semantic tokens and Tailwind classes so `futuristic` and `minimal` remain distinct without theme-name branches, inline styles, or new UI libraries.
- [x] 6.4 Extend i18n parity/visible-text and theme-token regression tests to cover the new settings and auxiliary surface files.

## 7. End-to-End and Native Verification

- [x] 7.1 Add Playwright coverage for the Web-renderable collapsed/menu/chat surfaces, active/no-session states, actions, persisted configuration, concurrent-send guard, both languages, and both visual styles.
- [x] 7.2 Run and document a Windows Tauri smoke matrix for enable/disable, always-on-top, taskbar visibility, X-to-hide, minimize, restore actions, explicit exit, drag persistence, monitor/DPI changes, hidden-main streaming, and failure recovery.
- [x] 7.3 Run `npm run lint`, `npm run test`, `npm run build`, and the Playwright suite; resolve all failures.
- [x] 7.4 Run `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo clippy --manifest-path src-tauri/Cargo.toml`; resolve all failures without production `unwrap()` or `expect()`.
- [x] 7.5 Run `openspec validate "add-desktop-floating-assistant" --strict` and `openspec validate --specs --strict`, then confirm every acceptance scenario is covered by automation or the documented native smoke matrix.

## 8. Verification Remediation

- [x] 8.1 Validate and compose every send request against the session-authoritative chat configuration before message insertion or CLI argument construction, with matching Web behavior and regression tests.
- [x] 8.2 Complete mini-chat lifecycle text/status semantics, no-session New Session action, direct collapse behavior, and targeted browser coverage.
- [x] 8.3 Persist a stable monitor-aware bottom-right anchor across native surface sizes and add geometry regression tests.
- [x] 8.4 Re-run the full frontend, Rust, Playwright, and OpenSpec validation suites and update verification evidence.
