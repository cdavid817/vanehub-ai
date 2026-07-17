## Context

The requested feature is a global outbound network proxy for VaneHub itself. It is different from CC Switch's local AI routing proxy: this feature does not transform provider protocols or take over Claude/Codex/Gemini configuration files. It configures how VaneHub-managed network requests leave the application.

The closest reference in `D:\cdavid\Documents\code\cc-switch` is the `GlobalProxySettings` surface and its related `global_proxy` command/service code. The useful interaction pattern is URL entry, optional authentication, local proxy scanning, test connection, clear, and save. VaneHub should adapt that pattern to its own settings architecture rather than copying CC Switch's React Query, shadcn, or local routing system.

VaneHub already has a shared settings model, desktop/Web adapters, Basic Configuration UI, SQLite-backed Tauri settings, and a centralized native command construction helper in `src-tauri/src/command_safety.rs`. Those existing boundaries should carry the change.

## Goals / Non-Goals

**Goals:**
- Add a Basic Configuration Network Proxy section.
- Support `http`, `https`, `socks5`, and `socks5h` proxy URLs.
- Support optional username/password authentication in the UI.
- Save authenticated proxy configuration as a single proxy URL after validation.
- Apply the saved proxy immediately to new VaneHub native outbound requests.
- Apply the saved proxy to new VaneHub-launched npm, CLI, SDK, and MCP-related subprocesses through environment variables.
- Support an editable `NO_PROXY` bypass list with localhost and loopback defaults.
- Provide desktop-only proxy test and local proxy scan actions.
- Keep Web/mock behavior honest and non-native.
- Preserve i18n parity for zh-CN and en.
- Keep the UI coherent in both `futuristic` and `minimal` styles.

**Non-Goals:**
- No OS-level system proxy modification.
- No interception of traffic from external terminals or tools not launched by VaneHub.
- No guarantee for WebView internal resource loading or browser network requests in Web/mock mode.
- No provider protocol conversion, local AI routing, request logging, failover, or usage accounting in this change.
- No first-version secret-store integration for proxy credentials.
- No forced restart or mutation of already-running subprocesses.
- No user-configurable test URL unless later feedback shows the built-in targets are insufficient.

## Terminology

- **Network proxy / outbound proxy:** A user-configured upstream proxy such as `http://127.0.0.1:7890` or `socks5://127.0.0.1:1080`.
- **Local routing proxy:** A protocol-aware local AI proxy like CC Switch's provider routing system. This is out of scope.
- **VaneHub-managed traffic:** Native runtime HTTP requests and child processes launched by VaneHub. This is the supported meaning of "all traffic" for this first version.

## Decisions

### Decision: Define "all traffic" as VaneHub-managed traffic

The first version will route VaneHub native outbound requests and VaneHub-launched child process network activity through the configured proxy. This includes newly launched npm, CLI, SDK, and MCP-related processes when they honor standard proxy environment variables.

This does not include OS-wide traffic, external terminals, existing long-running child processes, WebView internals, or arbitrary third-party processes outside VaneHub control.

Alternatives considered:
- Promise true system-wide interception: rejected because it would require OS proxy changes or packet-level interception outside the app's scope and safety model.
- Only proxy Rust HTTP clients: rejected because npm/CLI/SDK operations are a major source of network activity in this product.

### Decision: Store proxy authentication in the proxy URL for the first version

The UI may collect proxy address, username, and password separately, then merge them into a single URL such as `http://user:pass@127.0.0.1:7890` before saving. Logs and display surfaces must mask credentials.

This keeps the settings model simple and matches common proxy client APIs. The design explicitly records this trade-off so a later change can move credentials into a secure store without changing the user-facing concept.

Alternatives considered:
- Store username/password as separate settings: rejected for first version because most runtime APIs consume a single proxy URL and this increases validation and masking complexity without adding real security.
- Integrate OS credential storage now: deferred because it is valuable but substantially expands platform-specific scope.

### Decision: Keep Web/mock non-native

Web/mock mode can display the proxy setting and persist mock values through localStorage if useful for UI preview, but scan, test, and actual proxy application are desktop-only. Disabled states or runtime-unavailable messages should make that limitation visible.

Alternatives considered:
- Simulate successful proxy tests in Web/mock: rejected because it creates misleading behavior.
- Hide the section entirely in Web/mock: rejected because layout and localization should remain previewable.

### Decision: Apply proxy immediately to future work only

Saving a valid proxy URL updates persistent settings and runtime state immediately. New native requests and newly spawned subprocesses use the new proxy. Already-running processes are not restarted or reconfigured.

Alternatives considered:
- Require app restart: rejected because users expect settings to take effect immediately.
- Restart active sessions/processes automatically: rejected because it is disruptive and could destroy in-flight work.

### Decision: Use standard proxy environment variables for child processes

When a proxy is enabled, VaneHub-launched subprocesses should receive `HTTP_PROXY`, `HTTPS_PROXY`, and `ALL_PROXY`. Lowercase variants may also be set if needed for compatibility. `NO_PROXY` and `no_proxy` should be populated from the editable bypass setting.

The injection point should be centralized in native command construction, likely `command_safety`, so npm, CLI, SDK, and future process-launch paths do not each implement their own proxy logic.

Alternatives considered:
- Patch each npm/CLI/SDK call individually: rejected because it is easy to miss future process launches.
- Write npm-specific config files: rejected for first version because it mutates external tool state and does not cover all child process types.

### Decision: Support editable `NO_PROXY` with localhost and loopback defaults

The first version should expose an editable bypass list that maps to `NO_PROXY` / `no_proxy`. Its default should avoid proxying `localhost`, `127.0.0.1`, `::1`, and common local-only host patterns through the outbound proxy. This protects local MCP servers, local dev servers, OAuth callbacks, and app-internal loopback flows.

The user request says "all traffic", but routing loopback traffic through an upstream proxy is more likely to break local workflows than help. The default should be visible and editable so advanced users can remove or extend bypass entries when needed.

Alternatives considered:
- No bypass at all: rejected because it can break local MCP and local development flows.
- Hidden hard-coded bypass only: rejected because users need to inspect and adjust proxy routing exceptions.

### Decision: Built-in test targets for first version

The desktop proxy test action should use a small built-in target list and consider the proxy usable if any target succeeds. Candidate targets are public HTTP test or provider endpoints such as `https://httpbin.org/get`, `https://api.openai.com`, and `https://api.anthropic.com`.

The implementation should keep targets easy to change. The first version will not replace these with region-specific or China-optimized targets. A custom test URL can be added later if users need region-specific diagnostics.

Alternatives considered:
- Ask user for a test URL now: deferred because it increases UI complexity and validation surface.
- Use only one target: rejected because a single endpoint can be blocked or intermittently unavailable.

### Decision: Keep UI compact and aligned with Basic Configuration

The Network Proxy section should use existing VaneHub primitives and classes: `SectionPanel`, `Button`, `ucd-input`, status messages, and lucide icons. It should not introduce a new component library or CC Switch's shadcn/React Query conventions.

The section should work in the existing Basic Configuration grid and remain legible in both `futuristic` and `minimal` themes.

Alternatives considered:
- Copy CC Switch UI directly: rejected because VaneHub has different UI primitives and theme tokens.
- Add a separate proxy page: rejected because this is a basic app-wide setting, not a full local routing feature.

## Proposed User Experience

The Basic Configuration page gains a Network Proxy section with:

- Proxy URL input with placeholder examples such as `http://127.0.0.1:7890` and `socks5://127.0.0.1:1080`.
- `NO_PROXY` input with default localhost and loopback bypass values.
- Optional username input.
- Optional password input with show/hide control.
- Scan action for common local proxy ports.
- Test action for the current composed proxy URL.
- Clear action.
- Save action, disabled when there are no changes or a save is running.
- Status area for last test result, validation errors, scan results, and Web/mock unavailable notes.

Detected local proxies can appear as compact selectable buttons. Selecting one fills the proxy URL draft and marks the form dirty.

## Native Runtime Model

The settings service should expose a persisted `networkProxyUrl` value. Desktop saving should:

1. Normalize empty or whitespace input to direct connection.
2. Validate URL syntax and supported scheme.
3. Persist the value only after validation succeeds.
4. Apply the value to runtime proxy state.
5. Return normalized settings to the frontend.

The settings service should also expose a persisted `networkProxyBypass` value. Desktop saving should:

1. Normalize whitespace and separators into a stable comma-separated list.
2. Reject control characters.
3. Persist the value after validation succeeds.
4. Apply the value to runtime proxy bypass state.
5. Return normalized settings to the frontend.

Proxy application should be reusable by:

- Native HTTP clients added in this or future changes.
- Subprocess environment injection via command construction helpers.
- Proxy test and scan commands.

Any diagnostic logs must mask credentials before writing or displaying proxy URLs.

## Web/mock Runtime Model

The Web/mock adapter should remain interface-compatible. It may persist `networkProxyUrl` in localStorage as part of mock settings. Desktop-only operations should either be disabled by UI metadata or return a clear runtime-unavailable error.

The Web/mock adapter may also persist `networkProxyBypass` for preview and form-state behavior. It must not imply that browser requests honor that bypass list.

The Web/mock adapter must not pretend that browser requests are routed through the configured proxy.

## Validation

Proxy URL validation should accept:

- Empty string for direct connection.
- `http://host:port`
- `https://host:port`
- `socks5://host:port`
- `socks5h://host:port`
- The same schemes with URL-encoded username/password.

Validation should reject:

- Unsupported schemes.
- Control characters.
- Missing host.
- Malformed URLs.
- Credential values that cannot be represented safely in a URL.

`NO_PROXY` bypass validation should accept a comma-separated list of hostnames, domains, IP addresses, CIDR-like values if supported by the underlying client, and wildcard/domain patterns commonly accepted by proxy-aware tooling. The first version should reject control characters and normalize empty input to an empty bypass list. The default value should include localhost and loopback entries.

## Testing Strategy

- Frontend settings normalization tests for `networkProxyUrl` defaults and invalid values.
- Frontend settings normalization tests for `networkProxyBypass` defaults, empty values, and invalid control characters.
- Web/mock settings tests proving save/restore behavior and desktop-only operation errors.
- Basic Configuration rendering tests for Network Proxy text, disabled Web/mock actions, and localized labels.
- i18n parity tests for zh-CN and en keys.
- Rust unit tests for proxy URL validation and credential masking.
- Rust tests for settings save rejection on invalid proxy URLs.
- Rust tests for child process proxy and `NO_PROXY` environment injection via the centralized command helper.
- Optional Rust async tests for proxy test command behavior using invalid URLs and a controlled failure path.
- OpenSpec validation for the new change.

## Risks / Trade-offs

- [Risk] Users interpret "all traffic" as OS-wide traffic. Mitigation: document the supported scope as VaneHub-managed traffic in UI copy and specs.
- [Risk] Proxy credentials are stored in SQLite as part of the URL. Mitigation: mask in logs/UI and document future secret-store migration.
- [Risk] Some tools ignore standard proxy environment variables. Mitigation: use standard env vars first and document tool-specific follow-ups if discovered.
- [Risk] Editable `NO_PROXY` values may be malformed or tool-specific. Mitigation: normalize simple comma-separated values, reject unsafe characters, and document that exact matching behavior depends on the client/tool honoring `NO_PROXY`.
- [Risk] Built-in test targets are blocked in some regions. Mitigation: try multiple generic/provider targets, keep the target list easy to revise, and keep custom test URL as a future extension. Do not replace the first-version targets with region-specific defaults.
- [Risk] Existing process-launch paths bypass `command_safety`. Mitigation: implementation must audit production `Command::new` usages and move them behind centralized helpers where appropriate.

## Migration Plan

1. Add OpenSpec requirements for the app settings model and Basic Configuration UI.
2. Extend frontend settings types, defaults, normalization, and service contracts.
3. Add desktop adapter operations and Web/mock behavior.
4. Extend Rust settings model, validation, and persistence.
5. Add native proxy runtime state, bypass state, and child-process environment injection.
6. Add Basic Configuration Network Proxy UI and translations.
7. Add tests for frontend, Rust validation, child-process env injection, and i18n.
8. Run OpenSpec, frontend, and Rust verification commands.

Rollback is additive: remove or hide the Network Proxy section and stop applying proxy runtime state while leaving the stored setting inert until a later migration or cleanup.

## Future Optimization Notes

- Move proxy credentials into OS credential storage or an encrypted app secret store.
- Add custom proxy test URL.
- Add per-operation proxy diagnostics showing whether a request used direct or proxy mode without exposing credentials.
- Add import from common system proxy settings if there is a safe, platform-specific path.
- Add proxy mode metadata for tools that need npm-specific or git-specific configuration beyond standard environment variables.

## Open Questions

- Should lowercase proxy env vars be set in addition to uppercase for all child processes, or only for compatibility-sensitive tools?
- Should built-in test targets prefer provider APIs or neutral connectivity endpoints?
- Should the UI show the saved URL with credentials fully hidden, or show username while hiding only password?
