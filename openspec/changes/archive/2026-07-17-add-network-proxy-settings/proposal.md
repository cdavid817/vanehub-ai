## Why

VaneHub needs a user-configurable outbound network proxy so desktop users can route VaneHub-managed network activity through a local or remote proxy server. This is especially useful when CLI, SDK, npm, or native runtime requests need the same connectivity path.

The feature belongs in Basic Configuration because it is a cross-cutting application setting rather than a provider-specific or local routing feature. It should follow the existing settings service boundary and keep React components decoupled from Tauri commands.

## What Changes

- Add a Network Proxy section to the Basic Configuration page.
- Persist a global outbound proxy URL in the shared settings model.
- Support HTTP, HTTPS, SOCKS5, and SOCKS5H proxy URLs, including optional username/password authentication.
- Provide desktop runtime actions to test a proxy URL and scan common local proxy ports.
- Apply the saved proxy immediately to new VaneHub native outbound requests and new VaneHub-launched child processes.
- Inject proxy environment variables into VaneHub-launched npm, CLI, SDK, and MCP-related subprocesses through a centralized native command creation path.
- Provide an editable `NO_PROXY` bypass list, defaulting to localhost and loopback values so local MCP servers, dev servers, and internal callbacks do not accidentally route through the proxy.
- Keep Web/mock behavior explicit: the setting can be displayed or saved for preview purposes, but actual proxying, testing, and scanning are desktop-only.
- Add synchronized zh-CN and en translation keys for the Network Proxy UI.

## Capabilities

### Modified Capabilities

- `app-settings`: Extend the common settings model with persisted network proxy configuration, editable bypass configuration, validation, restoration, desktop runtime application, and Web/mock behavior.
- `settings-center-ui`: Add Basic Configuration network proxy and `NO_PROXY` controls, localized text, desktop-only test/scan states, and visual consistency across both registered UCD styles.

## Impact

- Frontend settings types, normalization, defaults, and settings service contracts need a new network proxy field and desktop-only helper operations.
- `tauri-settings-client.ts` and `web-settings-client.ts` must stay interface-compatible.
- Basic Configuration UI needs a compact proxy configuration section matching existing `SectionPanel`, `Button`, `ucd-input`, status, and icon conventions.
- Rust settings persistence needs to validate and store the proxy URL.
- Rust settings persistence needs to validate and store the editable `NO_PROXY` bypass list.
- Native runtime needs a reusable proxy application point for future HTTP clients.
- VaneHub-launched subprocesses should receive `HTTP_PROXY`, `HTTPS_PROXY`, and `ALL_PROXY` when a proxy is configured, plus the configured `NO_PROXY` bypass value.
- Tests should cover settings normalization, proxy URL validation, bypass validation, Web/mock unavailable behavior, child-process environment injection, i18n parity, and Basic Configuration rendering.
