## Why

Skills can describe workflows but cannot safely contribute executable tools to an agent session. A bounded runtime is needed so trusted Skill packages can extend native API agents without executing arbitrary host scripts, bypassing permissions, or leaking capabilities across Skill and session lifecycles.

## What Changes

- Add a versioned Skill tool manifest and discovery pipeline for declarative tools and optional WebAssembly modules stored with a Skill package.
- Register discovered tools under collision-resistant, Skill-scoped identifiers and expose only the tools authorized for the effective Skill revision and active execution context.
- Execute declarative tools through existing registered tool operations and WebAssembly tools in a capability-based sandbox with schema validation, cancellation, time, memory, fuel, host-call, recursion, input, and output limits.
- Reject Python, shell, native binaries, unrestricted WASI imports, and executable Overlay content as Skill tool implementations.
- Route every delegated host operation through the existing tool execution and unified permission boundaries; a Skill trust decision never grants an operational permission.
- Normalize every third-party Skill tool request into one versioned permission manifest covering separate filesystem read/write scopes, network origins, structured process commands, secret capabilities, and centrally capped resources. A declaration is only an upper bound and never an automatic grant.
- Reuse the existing canonical filesystem boundary, managed network/proxy client, credential boundary, backend task lifecycle, and permission approval broker for host capabilities, including traversal, symlink, redirect, secret, process-tree, output, and concurrency containment.
- Classify package provenance as Built-in, Verified, Community, Local, or Untrusted while keeping provenance, executable trust, and operation authorization as independent decisions.
- Add per-revision trust, integrity, enablement, validation, quarantine, lifecycle, usage, and audit state for Skill-contributed tools.
- Add a Skill Tools management surface behind the frontend service boundary. The desktop runtime can validate and execute supported modules; the Web adapter reports execution as unsupported without claiming native capability.

## Capabilities

### New Capabilities

- `skill-tool-runtime`: Defines Skill tool packaging, discovery, trust, isolated execution, lifecycle, observability, and failure containment.

### Modified Capabilities

- `skill-management`: Extends effective Skill revisions with tool manifests, integrity state, and lifecycle-coupled tool availability.
- `agent-tool-execution`: Adds Skill-contributed tool catalog assembly, dispatch, transcript visibility, cancellation, and execution limits.
- `permissions-core`: Adds Skill tool principals and capability-to-resource/action evaluation without implicit grants.
- `permissions-approval`: Presents Skill provenance and delegated operations in approval requests and preserves fail-closed behavior.
- `settings-skill-management-ui`: Adds tool inspection, trust, enablement, validation, quarantine, and runtime diagnostics to Skill management.

## Impact

- Desktop/native: adds Skill tool domain, manifest validation, registry integration, a declarative dispatcher, an optional WebAssembly runtime adapter, trust/integrity persistence, Tauri commands, and unified logging events.
- Frontend: extends `AgentService` and both Tauri and Web adapters with Skill tool management contracts; React components remain independent of direct Tauri invocation.
- Dependencies: introduces a pinned WebAssembly runtime and JSON Schema validation library only after supply-chain review; executable modules remain optional and declarative tools remain available without them.
- Security: expands the executable surface but keeps filesystem, process, network, environment, secrets, and existing tools inaccessible unless explicitly declared, admitted by the relevant native boundary, and independently allowed by policy. CPU and memory hard isolation is claimed only for the bounded WebAssembly runtime; native host operations use explicit quotas and cancellation rather than an unsupported cross-platform isolation claim.
- Web runtime: supports honest inspection/mock states but does not execute local Skill modules.
