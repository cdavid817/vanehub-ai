## Context

See `proposal.md`. `src-tauri/src/platform/network/proxy.rs` owns a process-wide `NetworkProxyState` of a URL and a bypass string, defaulting to an empty URL and `localhost,127.0.0.1,::1`. Four constructors build `reqwest` clients from it, and each repeats the same shape:

```rust
if !state.url.is_empty() {
    let proxy = reqwest::Proxy::all(&state.url)?.no_proxy(NoProxy::from_string(&state.bypass));
    builder = builder.proxy(proxy);
}
```

The `else` branch is the defect. `reqwest::ClientBuilder` discovers a proxy on its own unless told not to, so an untouched builder is not a direct-connection client.

## Goals / Non-Goals

**Goals:**

- Make the empty-URL case mean what the settings model says it means.
- Apply the bypass list in both routing modes.
- Express the decision once so the four constructors cannot diverge.

**Non-Goals:**

- Changing the settings UI, the persisted shape, or the default bypass value.
- Proxying traffic VaneHub does not manage, or reconfiguring already-running subprocesses.
- Adopting the OS bypass list. Its syntax differs from `NoProxy`'s, and honouring it would reintroduce the dependence on external configuration this change removes.

## Decisions

### 1. Suppress discovery rather than pass through OS configuration

In direct-connection mode the builder gets an explicit `no_proxy()`, which disables `reqwest`'s discovery. The alternative — reading the OS proxy and re-applying it with VaneHub's bypass list — was rejected because it keeps the routing dependent on a source the user cannot see in VaneHub, cannot be asserted in a test that does not mutate machine state, and still contradicts the documented scope. Explicit configuration is the only routing input.

The visible cost is real: a user behind a corporate proxy who never opened VaneHub's proxy setting is currently carried by OS discovery and will lose that. The mitigation is a release note and the fact that the setting already exists; the alternative is keeping behaviour that is undeclared and cannot be tested.

### 2. One helper, four call sites

A single `apply_proxy_routing(builder)` decides between configured proxy and suppressed discovery, and both blocking and asynchronous builders route through it. The current duplication is what let the four constructors share one defect, so collapsing it is part of the fix rather than incidental tidying.

`reqwest`'s blocking and asynchronous `ClientBuilder` are distinct types with no shared trait, so the helper is generic over the two shapes or written twice against a small internal trait. Either is acceptable; what matters is that the decision itself exists once.

### 3. Test against a real loopback fixture with a proxy configured

The regression test sets a proxy URL pointing at a port with nothing listening, then issues a request to a loopback fixture covered by the bypass list, and asserts the fixture receives it. A test that only inspects the builder would pass against the current code, because the defect is in what `reqwest` does with a builder that was never told anything.

State is process-wide, so proxy-mutating tests must not run concurrently with each other; they serialise through a mutex the way other global-state tests in this crate do.

## Risks / Trade-offs

- [Users relying on OS proxy discovery lose it silently] → Release note, and the failure mode is a connection error rather than data loss or a wrong result.
- [Process-wide state makes tests order-sensitive] → Serialise the proxy-mutating tests and restore prior state on exit.
- [`no_proxy()` also suppresses environment variables] → Intended. `no_proxy` env vars were the workaround for this defect, not a feature; VaneHub's own bypass list replaces them for managed traffic. Subprocess environment behaviour is unchanged and still governed by the existing requirement.

## Migration Plan

1. Add the shared routing helper and switch the four constructors to it, preserving today's behaviour when a proxy URL is configured.
2. Apply the bypass in direct-connection mode and suppress discovery.
3. Add the loopback regression tests and confirm the previously hanging suite passes with no `no_proxy` prefix.
4. Roll back by restoring the `if !state.url.is_empty()` guard; no persisted data or schema is involved.
