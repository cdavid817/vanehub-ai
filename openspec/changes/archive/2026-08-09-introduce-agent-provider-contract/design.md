## Context

See `proposal.md` for motivation. The current code already has useful seams: `AgentRegistryRepository` describes selectable Agents, `AgentProcessGateway` and `AgentTerminalGateway` isolate application services from native execution, `RuntimeAgentProcessAdapter` owns headless child processes, `PortablePtyAgentTerminalRuntime` owns retained PTYs, and `ProviderOutputEvent` normalizes a useful subset of structured output.

The missing seam is one level lower. `providers/invocation.rs`, `providers/output.rs`, terminal usage dispatch, Session chat configuration, tooling catalogs, and several frontend files still select behavior from literal Agent ids. The source document also predates `antigravity-cli`; the repository now has five built-in CLI Agents, and its addition touched many independent enumerations.

The existing `AgentSession` application model represents a VaneHub Session snapshot. Generations are per-turn subprocesses while terminals are retained PTY processes, so a second stateful `AgentSession` trait would create ambiguous ownership and naming without replacing either lifecycle.

## Goals / Non-Goals

**Goals:**

- Add the smallest provider contract that can become the single runtime-strategy lookup point.
- Reuse the Agent catalog, process gateways, terminal gateway, normalized events, and tooling boundaries already present.
- Represent capability differences explicitly and preserve opaque resume ids.
- Introduce five compatibility registrations with contract and architecture tests.
- Leave every current runtime path behavior-compatible while enabling later strangler migrations.

**Non-Goals:**

- No new CLI, dynamic loading, SDK, marketplace, or UI redesign.
- No common process supervisor rewrite, full `AgentEvent` redesign, terminal usage rewrite, or provider migration.
- No new Tauri command, frontend DTO, or SQLite column.
- No replacement of the Sessions aggregate, Generation lifecycle, retained-terminal lifecycle, Agent catalog, or API Agent adapters.

## Decisions

### D1. Place provider concepts in `agent_runtime`, split by layer

Value types (`AgentProviderId`, `ProviderMetadata`, `ProviderCapabilities`, `ProviderReadinessPrerequisites`, and `ProviderSessionRef`) belong in `agent_runtime::domain`. The behavioral contract and registry belong in the application boundary. Concrete built-in CLI compatibility providers belong in infrastructure.

This follows the existing bounded context instead of creating a new top-level `cli` or `provider` context. A standalone context would have to depend back on Agent Runtime request/event types and would invert ownership.

### D2. Keep the Agent catalog and provider registry distinct

The Agent catalog remains the product-facing source for display name, launch modes, availability projection, origin, and user selection. The provider registry maps a stable Agent id to runtime strategy and capability declarations. During composition, every registered built-in CLI provider is checked against its Agent catalog entry; the provider does not seed a second Agent row.

`AgentProviderId` initially wraps the same stable string as `AgentId`. It is deliberately named as a runtime id to distinguish it from chat/API `provider_id`, which identifies a model vendor or endpoint.

Alternative: merge runtime objects into `AgentDefinition`. Rejected because the catalog is persisted/projected and includes dynamically registered API Agents, while runtime strategies contain behavior and dependencies.

### D3. Use a stateless provider contract, not a new `AgentSession` trait

The first contract exposes metadata, capabilities, readiness prerequisites, and provider-specific generation/terminal preparation and decoding collaborators. It does not own a long-lived Session:

```text
AgentProvider
├── descriptor and capabilities
├── readiness prerequisites
├── generation strategy (compatibility delegate initially)
├── terminal strategy (optional)
└── output decoder factory
```

The existing Sessions context owns VaneHub Session persistence; Generation owns per-turn execution state; `PortablePtyAgentTerminalRuntime` owns retained PTY state. Later work can introduce a differently named runtime handle if an API/AppServer provider proves it necessary.

Alternative: adopt the source document's `AgentSession` trait immediately. Rejected because it conflates three lifecycles and collides with the existing application model.

### D4. Build a deterministic static registry in the composition root

The Tauri composition root constructs an immutable `Arc<ProviderRegistry>` and injects it into runtime adapters. Registration order is explicit and listing is stable. Duplicate ids and invalid declarations fail bootstrap; lookups return a classified unsupported-provider error. No global mutable singleton or dynamic library loading is introduced.

The five compatibility providers delegate to the current helpers so the contract lands before migration. User-created API Agents continue through `CompositeAgentProcessGateway` and `ApiAgentProcessAdapter`; moving API implementations into the provider registry is a later change.

### D5. Capabilities are typed baseline declarations

Capabilities use typed values rather than an expanding set of free-form tags. The first declaration covers interaction mode, resume, structured output, terminal support, usage quality, model selection, reasoning, permissions, and sandbox behavior. Existing free-form `capability_tags` remain the public catalog projection until the frontend change.

Capabilities describe the provider implementation's baseline contract. Installed-version checks remain part of readiness; a later version-aware descriptor can reduce effective capabilities without changing the type. Encoding per-version matrices now would be speculative because current discovery does not expose a reliable semantic-version capability map.

### D6. Keep discovery and configuration in `tooling`

Executable discovery, install source, version probing, native configuration files, global CLI profiles, and parameter definitions stay under `tooling::cli`, `tooling::cli_config`, and `tooling::cli_parameters`. The provider declares readiness prerequisites and consumes resolved executable/profile data; it does not search PATH or edit config files.

The existing parameter catalog is not copied into `ProviderMetadata` in this change. A later metadata-driven UI change can expose the tooling-owned schema through the service boundary. This avoids creating two authorities while the backend and frontend parameter catalogs are already duplicated.

### D7. Evolve the existing process and event boundaries

The future shared process runtime belongs in `agent_runtime::infrastructure`, reusing `platform::process` for OS-level child management. There is no evidence yet that a separate bounded context is needed. PTY and structured stdio remain sibling transports; provider strategies select a transport, while the shared runtime owns spawn, I/O, interruption, cleanup, and logging.

The existing application `AgentEvent` is an integration/application event consumed by persistence and the frontend, not a domain event. `GenerationProcessEvent` and `ProviderOutputEvent` remain compatibility layers in this change. Full normalization, including a raw event escape hatch, is deferred to `normalize-agent-provider-events`.

### D8. Wrap current resume storage without migration

`ProviderSessionRef { provider_id, external_id }` is an Agent Runtime value. The Sessions context continues persisting the nullable `runtime_session_id` string, and the sessions gateway reconstructs the provider id from the owning Session's Agent id. Provider-specific semantics never enter the Sessions domain.

Metadata is intentionally omitted from the first value because no current provider needs persisted auxiliary fields. If a future provider requires them, a dedicated persistence change can add a versioned JSON field instead of silently overloading the string.

### D9. Introduce a focused provider error boundary

The contract defines invalid metadata, duplicate registration, unsupported provider, unsupported capability, and provider preparation failures. Infrastructure maps these into `AgentRuntimeApplicationError`; Tauri continues mapping application errors to safe strings. Existing process/auth/protocol failures remain unchanged until the error-normalization change, preventing a broad error rewrite here.

### D10. Preserve public APIs and enforce dependency direction

Existing Tauri commands and `AgentService` methods remain unchanged, and Web/mock code receives no new requirement in this change. There is no persistence migration.

`src-tauri/tests/architecture.rs` gains source-level dependency checks preventing Sessions domain/application and provider-neutral Agent Runtime application modules from importing concrete provider modules or matching built-in ids. Provider fixtures, composition, tooling catalogs, and explicitly provider-specific modules are allowlisted. A later frontend change adds equivalent guardrails after descriptors cross the service boundary.

## Risks / Trade-offs

- [Risk] Compatibility providers could become wrappers that never remove central id matches. → Each follow-up provider migration has acceptance criteria that delete its branch from shared invocation/decoder code.
- [Risk] Agent id and model-provider id terminology remains confusing. → Use `AgentProviderId` in Rust runtime code and document that API/model `provider_id` is a separate concept.
- [Risk] Static capability declarations can become stale as installed CLIs evolve. → Keep readiness separate and add effective/version-qualified capabilities only after real version evidence exists.
- [Risk] Source-scanning architecture tests can produce false positives. → Limit them to production modules, parse string literals conservatively, and keep explicit narrow exceptions close to provider-specific code.
- [Risk] The active `add-gemini-cli-terminal-usage-tracking` change overlaps terminal usage code. → This change does not edit terminal usage behavior; the later usage-strategy change depends on that work being completed or reconciled.

## Migration Plan

1. Add provider value types, errors, registry, and unit tests without wiring runtime behavior.
2. Add compatibility provider declarations for all five built-in CLI Agents and validate them against the existing Agent catalog.
3. Inject the immutable registry at the desktop composition root.
4. Route only provider resolution/descriptor lookup through the registry; continue delegating execution to current invocation, parser, process, terminal, and usage helpers.
5. Add architecture tests and behavior-parity tests using the existing invocation/output fixtures.
6. Run the repository's full validation suite. Rollback removes the injected registry and compatibility declarations; no data rollback is required.

## Open Questions

None for this change. Version-qualified capabilities, auxiliary provider-session metadata, and dynamic API-provider registration are intentionally deferred follow-ups rather than unresolved first-change decisions.

