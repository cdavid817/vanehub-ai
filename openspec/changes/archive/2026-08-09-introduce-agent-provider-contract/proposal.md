## Why

VaneHub already routes CLI generations through shared process and event boundaries, but provider identity still controls invocation, output parsing, terminal usage, session configuration, and frontend catalogs in multiple hard-coded branches. The recently added fifth built-in CLI, Antigravity, demonstrates that another CLI still requires coordinated edits across core runtime and UI layers, so a stable provider contract is needed before adding more agents.

## What Changes

- Introduce a small `AgentProvider` contract and static `ProviderRegistry` in `agent_runtime`, keyed by the existing stable Agent id.
- Define provider metadata, capabilities, readiness prerequisites, and an opaque provider-session reference without duplicating the existing Agent catalog or Session aggregate.
- Register compatibility providers for the five current built-in CLIs while delegating to existing invocation, decoder, process, terminal, and usage paths so behavior remains unchanged.
- Add contract and architecture tests that reject duplicate registrations, unknown providers, invalid capability declarations, and new provider-identity branching in provider-neutral Session/application modules.
- Keep existing Tauri commands, frontend service interfaces, SQLite schema, session rows, Web/mock behavior, and CLI behavior compatible.
- Do not add a new CLI, migrate process supervision, normalize the full event protocol, or make the frontend capability-driven in this change.

## Capabilities

### New Capabilities

- `agent-provider-runtime`: Defines provider resolution, metadata and capability declarations, compatibility registration, opaque provider-session references, and provider-neutral dependency constraints.

### Modified Capabilities

- None.

## Impact

- Desktop runtime: adds contracts and composition-root registration under `src-tauri/src/contexts/agent_runtime`; existing `AgentProcessGateway`, `AgentTerminalGateway`, provider invocation/parser helpers, and Session gateway remain the execution path during this change.
- Web runtime and frontend: no service or UI behavior changes; existing `AgentRegistryEntry.capabilityTags` and `listAgents()` remain the public projection.
- Data: no SQLite migration. The current nullable `sessions.runtime_session_id` remains intact and is interpreted through an opaque provider-session value inside the runtime boundary.
- Architecture: the Agent catalog remains the product-facing catalog, while the provider registry becomes the runtime-strategy registry. The term provider id in this contract means stable Agent id and is kept distinct from model/API `provider_id` fields.
- Dependencies: no new crate, npm package, dynamic plugin system, or marketplace.
