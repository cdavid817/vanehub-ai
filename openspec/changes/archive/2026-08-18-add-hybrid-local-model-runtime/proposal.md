## Why

VaneHub can call arbitrary OpenAI-compatible APIs, but OnePiece Profiles are limited to reviewed cloud catalog endpoints and the runtime has no explicit local/private endpoint trust, capability, context-budget, privacy, or fallback model. Users therefore cannot safely route eligible work to Ollama, LM Studio, vLLM, SGLang, or an enterprise-compatible service without misrepresenting readiness or risking an unintended cloud fallback.

## What Changes

- Extend the existing API-Agent and OnePiece Profile model with explicit runtime kind, endpoint, interface, model, optional authentication, timeout, privacy classification, declared/verified capabilities, and context-window value plus provenance and confidence.
- Add manual local/private OpenAI-compatible endpoints and an opt-in, localhost-only discovery operation that performs bounded readiness/model-metadata probes, never scans the LAN, sends source content, or executes a real task.
- Add visible, disableable rule routing from task class to preferred and fallback Profiles, with a persisted reason and a hard `local-only` policy that waits for user choice instead of falling back to cloud.
- Negotiate tool calling, image input, structured output, and reasoning-field support before request execution; unsupported capabilities fail or route according to policy without guessing from a model name.
- Feed the selected Profile's verified or conservative configured context budget into the existing Context Engine, label estimates, and prevent retry loops after a context-limit failure.
- Preserve missing provider usage as unavailable/estimated evidence without inventing billed cost.
- Extend the shared frontend service contract and both Tauri and Web/mock adapters, and add localized Profile, discovery, verification, capability, privacy, and routing controls to the existing OnePiece configuration surface.
- Add SQLite migration and compatibility handling for existing OnePiece Profiles and API Agents without changing their stable identities or current active selections.
- Add fake-server, negative-security, contract, UI/E2E, desktop localhost, visual, and deterministic streaming/performance coverage.

## Capabilities

### New Capabilities

- `hybrid-local-model-runtime`: Defines local/private endpoint Profiles, bounded localhost discovery, policy-aware Hybrid Routing, capability negotiation, privacy fallback, operation observability, and non-blocking streaming behavior.

### Modified Capabilities

- `api-agent-runtime`: Allows optional authentication for explicitly configured local/private endpoints and executes them through the existing API generation path with Profile metadata and negotiated capabilities.
- `onepiece-native-agent`: Extends OnePiece Profile management, discovery, verification, runtime selection, and settings UI to local/private endpoints and Hybrid Routing while preserving stable identity and adapter parity.
- `agent-context-engine`: Uses the routed Profile's effective context budget and provenance for evidence planning and bounded projection.
- `agent-context-measurement`: Represents configured conservative context capacity and provenance without inferring capacity from model names or fabricating utilization.

## Impact

- Desktop/native: the existing `agent_runtime` bounded context, its application ports/domain models, API process adapter, SQLite migrations/repository, async operation/logging integration, and Tauri commands.
- Frontend: `AgentService`, Tauri and Web/mock adapters, shared Agent contracts/types, OnePiece settings components, i18n resources, and responsive visual tests.
- Compatibility: existing catalog-backed Profiles and user-created API Agents retain their ids, active configuration, credentials, and behavior; new fields receive conservative defaults. No new bounded context, state library, UI library, provider-specific Agent branch, or external provider package loader is introduced.
- Security/network: probes are explicit, timeout-bounded, restricted to loopback for automatic discovery, redact credentials and response bodies, and never include repository/session content. Manual enterprise endpoints remain user-entered and are not LAN-discovered.
- Both desktop and Web runtimes change through the shared service boundary; Web/mock behavior remains deterministic and performs no network access.
