# Stop direct connection from inheriting the operating system proxy

## Why

`app-settings` already says an empty proxy URL means direct connection, that a default bypass list covers localhost and loopback traffic, and that VaneHub does not promise OS-wide proxying. The implementation does not deliver any of the three.

All four client builders in `src-tauri/src/platform/network/proxy.rs` apply the proxy and its bypass list only when `state.url` is non-empty. When it is empty they hand `reqwest` an unconfigured builder, and `reqwest` falls back to its own proxy discovery — environment variables, and on Windows the `Internet Settings` registry keys. So in the one case the settings model calls "direct connection", VaneHub silently adopts whatever proxy the operating system is configured for, and `DEFAULT_BYPASS` never reaches the client at all.

The consequence is not limited to outbound API traffic. VaneHub talks to local sidecars, MCP relays, and the permission hook bridge over loopback HTTP. On a machine with a system proxy, those requests are sent to the proxy instead of to `127.0.0.1`, and the bypass list that exists precisely to prevent this is inert.

This was found while verifying an unrelated change: on a developer machine with `ProxyEnable=1` and `ProxyServer=127.0.0.1:9999`, `cargo test` hung. Tests that bind an ephemeral loopback fixture and wait in `accept()` never receive the connection, because the client under test built through `blocking_http_client` and was routed to the proxy. `ProxyOverride` did list `127.*`, so the OS bypass list is not a reliable defence either — `reqwest` does not honour that wildcard form. Setting `no_proxy` explicitly made the same tests pass unchanged, which confirms the routing rather than the code was at fault.

## What Changes

- Apply the configured bypass list in direct-connection mode, not only when VaneHub has its own proxy URL, so loopback traffic reaches loopback.
- Make direct-connection mode mean direct connection: VaneHub-managed requests stop silently adopting OS or environment proxy configuration discovered outside the application's own setting.
- Cover the four client builders (`http_client`, `blocking_http_client`, `no_redirect_http_client`, `blocking_no_redirect_http_client`) with one shared decision so they cannot drift apart again.
- Add regression tests that a loopback request is not proxied while a system proxy is configured.

## Capabilities

### Modified Capabilities

- `app-settings`: States that direct-connection mode does not inherit externally discovered proxy configuration, and that the bypass list applies in every mode rather than only when a VaneHub proxy URL is set.

## Impact

- Native/desktop: `src-tauri/src/platform/network/proxy.rs` and every caller of its client constructors, which today includes the API process adapter, memory extraction gateway, OnePiece planning, and utility delegation.
- Behaviour: a user who relies on an OS-level proxy for outbound API traffic without configuring VaneHub's own proxy setting will lose that routing and must set the proxy in VaneHub settings. This is the intended correction — the current behaviour is undeclared, untestable, and already contradicts the documented scope — but it is a visible change and needs a release note.
- Local verification: removes the need for a `no_proxy` prefix when running the Rust suite on a machine with a system proxy.
