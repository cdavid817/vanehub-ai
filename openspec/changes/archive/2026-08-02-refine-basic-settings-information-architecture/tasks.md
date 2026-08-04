## 1. Shared Settings Presentation

- [x] 1.1 Refine the shared settings group and row primitives for compact intent-based sections and disclosures.
- [x] 1.2 Add synchronized zh-CN and en text for the new groups, default project directory, advanced disclosure, and reset confirmation.

## 2. Basic Configuration Information Architecture

- [x] 2.1 Recompose Basic Configuration into common preferences, startup and window behavior, workspace defaults, and advanced configuration.
- [x] 2.2 Add the default project directory field and persist `defaultFolderPath` through `SettingsProvider` in desktop and Web/mock runtimes.
- [x] 2.3 Refactor startup, floating-assistant, and folder-opener presentations into embeddable setting rows while preserving their service behavior.
- [x] 2.4 Move network proxy, log management, data management, and Node.js runtime information into the collapsed advanced disclosure.
- [x] 2.5 Move global reset to the page footer and require localized confirmation before invoking reset.

## 3. Verification

- [x] 3.1 Add or update component tests for group order, default folder persistence, disclosure state, accessible labels, and reset cancellation.
- [x] 3.2 Run `npm run lint`, `npm run test`, and `npm run build`.
- [x] 3.3 Run `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo clippy --manifest-path src-tauri/Cargo.toml`.
- [x] 3.4 Run `openspec validate refine-basic-settings-information-architecture --strict` and `openspec validate --specs --strict`.
