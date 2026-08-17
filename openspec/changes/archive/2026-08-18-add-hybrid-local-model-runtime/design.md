## Context

See `proposal.md` for motivation. OnePiece already owns multiple catalog-backed Profiles, model discovery and credential verification; API Agents already use the same Anthropic/OpenAI-compatible generation adapter; Context Engine capacity is resolved through a reviewed model catalog; token accounting already tolerates absent OpenAI-compatible usage. The missing work is a richer endpoint Profile and policy layer, not another provider runtime.

The change crosses frontend contracts, Tauri commands, `agent_runtime` domain/application/infrastructure, SQLite, async operations/logging, and settings UI. It must preserve existing Profile ids, credentials, active projections, generation events, command compatibility, and Web/mock behavior.

## Goals / Non-Goals

**Goals:**

- Generalize the existing Profile snapshot sufficiently for catalog, loopback, and manually trusted private OpenAI-compatible endpoints.
- Make routing, privacy, capability, and context-budget decisions deterministic before request construction.
- Keep network discovery metadata-only, opt-in, bounded, redacted, and loopback-only.
- Preserve responsive streaming and measured structural budgets.

**Non-Goals:**

- Implement Runner Abstraction, background Agents, remote workers, model installation, lifecycle management for Ollama/LM Studio/vLLM/SGLang processes, LAN discovery, or an external provider plugin loader.
- Add provider-specific generation adapters for local products, infer capabilities from model names, promise local security, or calculate unverified billing prices.
- Replace existing direct active-Profile selection; Hybrid Routing is an optional policy above it.

## Decisions

### 1. Keep ownership in `agent_runtime`

Endpoint Profiles, routing selection, capability admission, and generation snapshots are part of provider invocation and generation lifecycle, already owned by `agent_runtime`. Domain types enforce Profile invariants and privacy compatibility; application services coordinate repository, credential, discovery, operation, and diagnostic ports; infrastructure owns SQLite and HTTP.

Adding a `local_model_runtime` bounded context was rejected because it would duplicate Profile and provider invocation ownership and force cross-context access to private generation state.

### 2. Evolve the existing Profile contract additively

The Profile gains runtime kind (`cloud`, `local`, `private`), endpoint source (`catalog`, `configured`, `discovered`), authentication mode (`required`, `optional`, `none`), timeout, privacy class, capability states, and context metadata. Catalog Profiles continue resolving immutable endpoint data. Custom Profiles use the same OpenAI-compatible gateway and are validated as explicit endpoint records.

Existing OnePiece rows receive conservative defaults through a new forward-only migration. User-created API Agents gain equivalent snapshot defaults without changing stable ids or forcing multiple Profiles in this version. A separate parallel local-provider table was rejected because it would create two Profile concepts and conflicting activation rules.

### 3. Route before context collection and freeze one snapshot

Generation admission derives task class from an explicit request classification when available and otherwise uses a bounded deterministic classifier with an `unknown` result. It evaluates enabled ordered rules, readiness, capabilities, and privacy. The winning Profile is frozen with rule id/reason before Context Engine planning, credential resolution, request building, and accounting attribution.

If a fallback changes the Profile before provider contact, Context Engine planning is recomputed. There is no automatic mid-stream provider switch. This avoids leaking partial content, mixing attribution, and applying the wrong context budget.

### 4. Treat privacy as an admission invariant

Task policy is a required enum. `local-only` admits only `local` Profiles and never cloud fallback. `local-preferred` may use an explicitly configured compatible fallback; `cloud-allowed` follows the rule without location restriction. Failure produces a typed waiting outcome containing safe ids/reasons and no content.

Relying on UI warnings was rejected because routing can also be initiated by plans, loops, and future service consumers.

### 5. Use explicit capability states

Each capability is `supported`, `unsupported`, or `unknown`, with `configured` or `verified` provenance. Requirements are derived from the pending request (tools, images, structured output, reasoning). Admission either selects a compatible fallback or returns a typed error before provider contact. Optional request fields are omitted when unsupported.

Product-name and model-name lookup was rejected because OpenAI-compatible servers expose different builds and feature flags under identical ids.

### 6. Separate manual endpoints from automatic discovery

Manual entry accepts validated HTTP/HTTPS origins, including enterprise hosts, with explicit trust confirmation. Automatic discovery probes only a compiled loopback allowlist. Redirects are disabled or revalidated to remain loopback. Probe response bytes, model count, concurrency, and timeout are bounded. Discovery returns operation ids immediately and exposes queued/running/succeeded/failed state via existing operations patterns.

The probe prefers `/v1/models` and service metadata variants but never sends `/chat/completions`. LAN ranges, mDNS, port sweeps, and service startup are out of scope.

### 7. Scope capacity metadata to the Profile endpoint

Effective capacity priority is verified endpoint metadata, user-configured conservative value, then existing unknown-capacity policy. The immutable Profile snapshot carries value, provenance, confidence, reserve, and policy version into context measurement and planning. A context-limit response permits at most one already-authorized reduction attempt; it never causes an unlimited retry or a silent provider switch.

### 8. Preserve service and adapter symmetry

`AgentService` gains Profile metadata, discovery/verification operation, routing-rule, and preview methods. `tauri-agent-client.ts` alone maps them to commands; Web/mock uses deterministic in-memory state and synthetic operations. React components consume the service interface and shared types only. UI copy is localized across every registered locale and shared semantic tokens cover futuristic/minimal themes.

### 9. Reuse accounting, logging, and streaming paths

Local responses flow through the existing OpenAI-compatible SSE parser and chat events. Missing usage follows current estimation-quality rules and price remains absent without reviewed pricing. Discovery, routing, fallback, and capability outcomes emit content-free operation diagnostics through the unified log port. A fake HTTP server supplies model-list, malformed, timeout, context-limit, missing-capability, missing-usage, and large-stream fixtures.

## Risks / Trade-offs

- [OpenAI-compatible dialects differ] → Keep protocol parsing bounded, expose unknown capability states, and test multiple model-list/SSE shapes without product branches.
- [SSRF through manual URLs or redirects] → Validate scheme/credentials/host, distinguish manual trust, revalidate redirect targets, and keep auto-discovery loopback-only.
- [Privacy classification is user-configured] → Enforce it mechanically while avoiding claims that location proves security.
- [Migration misclassifies historical endpoints] → Default historical catalog matches from catalog provenance; all other endpoints become configured private/unknown, never automatically local.
- [Routing adds latency] → Use deterministic in-memory rule evaluation and structural benchmarks; network verification is never on implicit selection paths.
- [Large local streams pressure the renderer] → Reuse bounded batching/backpressure and add chunk-partition plus structural work-budget tests.

## Migration Plan

1. Add forward-only SQLite columns/tables and compatibility defaults; migration fixtures prove old catalog, legacy custom, active, inactive, and missing-credential rows retain behavior.
2. Deploy read/write support before enabling Hybrid Routing. Existing Profiles behave as direct active Profiles with routing disabled.
3. Add commands/adapters/UI and only enable user-created rules after validation and readiness checks.
4. Rollback code may ignore additive metadata while legacy fields remain intact; databases are not destructively downgraded. A later forward migration can clean unused rule records if required.
