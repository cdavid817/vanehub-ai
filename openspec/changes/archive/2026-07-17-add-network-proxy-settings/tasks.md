## 1. Settings Contracts

- [x] 1.1 Extend frontend `AppSettings` and `AppSettingKey` with `networkProxyUrl` and `networkProxyBypass`.
- [x] 1.2 Extend settings defaults, normalization, and validation for proxy URLs and bypass lists.
- [x] 1.3 Keep Tauri and Web/mock settings clients interface-compatible.
- [x] 1.4 Add service-level operations or metadata for desktop-only proxy test and local proxy scan behavior.
- [x] 1.5 Add frontend settings tests for defaults, invalid values, and Web/mock behavior.

## 2. Native Runtime

- [x] 2.1 Extend Rust settings model, validation, and SQLite key/value persistence for `networkProxyUrl` and `networkProxyBypass`.
- [x] 2.2 Add native proxy URL validation, bypass-list normalization, and credential masking helpers.
- [x] 2.3 Apply saved proxy and bypass settings to runtime proxy state after validation and persistence.
- [x] 2.4 Inject `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`, and configured `NO_PROXY` into VaneHub-launched child processes through centralized command creation.
- [x] 2.5 Audit production `Command::new` usage and move network-capable child process launches behind centralized helpers where needed.
- [x] 2.6 Add desktop commands for proxy test and local proxy scan behind the settings service boundary.
- [x] 2.7 Add Rust tests for proxy validation, bypass validation, masking, settings rejection, and child-process environment injection.

## 3. Basic Configuration UI

- [x] 3.1 Add a Network Proxy `SectionPanel` to the Basic Configuration page.
- [x] 3.2 Add proxy URL, `NO_PROXY`, username, password, show/hide password, scan, test, clear, and save interactions.
- [x] 3.3 Show local proxy scan results as selectable options.
- [x] 3.4 Show test result and validation errors without writing local logs directly from React.
- [x] 3.5 Keep Web/mock desktop-only actions disabled or clearly unavailable.
- [x] 3.6 Ensure the layout remains coherent in both `futuristic` and `minimal` themes.

## 4. Localization And Verification

- [x] 4.1 Add synchronized zh-CN and en translation keys for all Network Proxy and `NO_PROXY` labels, placeholders, actions, errors, and statuses.
- [x] 4.2 Add Basic Configuration rendering tests for the Network Proxy section.
- [x] 4.3 Run `openspec validate "add-network-proxy-settings" --strict`.
- [x] 4.4 Run `npm run test`.
- [x] 4.5 Run `npm run build`.
- [x] 4.6 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 4.7 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
