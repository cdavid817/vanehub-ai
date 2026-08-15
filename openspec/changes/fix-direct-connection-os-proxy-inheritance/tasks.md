## 1. Shared routing decision

- [ ] 1.1 Add one helper in `platform/network/proxy.rs` that decides between the configured proxy and suppressed discovery, covering both the blocking and asynchronous `reqwest` builder shapes.
- [ ] 1.2 Route `http_client`, `blocking_http_client`, `no_redirect_http_client`, and `blocking_no_redirect_http_client` through it, leaving behaviour unchanged while a proxy URL is configured.

## 2. Direct connection and bypass

- [ ] 2.1 Apply the configured bypass list in direct-connection mode instead of only when a proxy URL is set.
- [ ] 2.2 Suppress `reqwest` proxy discovery in direct-connection mode so no OS or environment proxy is adopted.

## 3. Tests

- [ ] 3.1 Prove a bypassed loopback request reaches its fixture while a VaneHub proxy URL points at a dead port.
- [ ] 3.2 Prove a non-bypassed request still uses a configured VaneHub proxy.
- [ ] 3.3 Prove direct-connection mode ignores an externally declared proxy, driving discovery through the environment so the assertion does not depend on machine state.
- [ ] 3.4 Serialise proxy-mutating tests against the process-wide state and restore the prior state on exit.

## 4. Documentation

- [ ] 4.1 Add a release note that OS-level proxy configuration is no longer inherited and that the proxy must be set in VaneHub settings.

## 5. Verification

- [ ] 5.1 Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo check --manifest-path src-tauri/Cargo.toml`.
- [ ] 5.2 Run the full Rust suite on a host with a system proxy enabled and no `no_proxy` prefix, confirming the loopback fixture tests no longer hang.
- [ ] 5.3 Run `npm run lint:ci`, `npm run test`, `npm run build`, and `npm run docs:check`.
- [ ] 5.4 Run `openspec validate fix-direct-connection-os-proxy-inheritance --strict` and `openspec validate --specs --strict`.
