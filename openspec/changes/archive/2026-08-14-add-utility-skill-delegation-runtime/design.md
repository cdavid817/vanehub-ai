## Context

See `proposal.md` for motivation. The effective Skill catalog already parses Utility metadata but marks every Utility definition unavailable, `load_skill` correctly refuses Utility bodies, and native API Agents already execute a closed tool catalog with correlated run events. The evidence change provides a fail-open `EvidenceEnvelopeSink`, but it must consume authoritative facts rather than infer delegation from provider output.

The design must preserve the tooling/agent-runtime context boundary, keep React behind the shared service interface, and avoid adding raw Utility content to evidence or unified logs.

## Goals / Non-Goals

**Goals:**

- Establish a single native admission boundary that resolves an exact effective Utility revision.
- Run bounded specialist attempts with deterministic lifecycle convergence and parent cancellation.
- Expose safe terminal facts to observability and evolution evidence.
- Report desktop and Web capability differences through shared contracts.

**Non-Goals:**

- Supporting Utility execution from unmanaged CLI sessions.
- Allowing nested Utility delegation in the first version.
- Treating Utility output as trusted or automatically applying changes.
- Persisting raw task input, instructions, tool arguments, or output in evidence storage.
- Adding a generic autonomous multi-agent scheduler.

## Decisions

### 1. Agent runtime owns execution; tooling owns exact Skill resolution

The tooling Skills application exposes a narrow resolution operation that accepts canonical workspace and Skill identity and returns an immutable execution snapshot: canonical id, effective revision, bounded instructions/resource metadata, trust, and source identity. The agent runtime owns admission, cancellation, limits, provider execution, and lifecycle publication.

This keeps filesystem/catalog policy in tooling while preventing tooling from depending on provider execution. A direct agent-runtime read of Skill files was rejected because it would duplicate overlay, shadowing, trust, and revision rules.

### 2. Use a fixed native tool and closed request/result contracts

Supported native API Agents receive `delegate_utility_skill`. Its request contains only Skill id, bounded task text, and optional limits capped by host policy. Its result contains delegation/attempt ids, terminal status, bounded safe summary, and counts. The provider cannot supply environment variables, paths, credentials, executors, or arbitrary configuration.

Reusing generic provider tool metadata was rejected because `delegation_id` alone does not prove a VaneHub Utility Skill or revision was executed.

### 3. Execute an isolated child generation with a restricted tool catalog

An admitted attempt creates a child execution context using the parent Agent's configured provider credentials and model route, but with a Utility-specific system instruction, task input, cancellation token, and restricted existing tool catalog. The first version removes the delegation tool from the child catalog, which enforces depth zero without relying on prompt compliance.

Spawning a shell process per Utility was rejected because Utility Skills are provider-guided specialists, not executable packages, and arbitrary scripts would expand the trust boundary.

### 4. Use a lifecycle state machine with idempotent terminal convergence

The application service owns `Admitted -> Running -> Terminal`. It generates a UUIDv7 delegation id and attempt id before execution, emits one started fact after the adapter accepts work, and accepts only the first valid terminal transition. Parent stop and timeout use the same cancellation primitive as native generation.

Terminal classifications are `succeeded`, `failed`, `cancelled`, `timed-out`, `limited`, and `refused`. Refusal before adapter start records no started fact; admitted attempts always converge to one terminal fact.

### 5. Keep content at the execution boundary and project metadata only

The child provider adapter may hold bounded instructions, task text, and response content transiently to perform work and return the safe bounded result to the parent generation. Observability, unified logs, and evolution evidence receive only ids, exact revision, timestamps/duration, terminal classification, counts, limit reason, and fidelity.

The existing evidence sanitizer remains defense in depth, not authorization to forward raw content. Installation-keyed workspace scope projection is reused.

### 6. Make catalog availability runtime-aware without changing Skill type

Utility definitions become available only when valid, enabled, effective, trusted under existing rules, and viewed through a native runtime with delegation support. `load_skill` continues to refuse them. Web/mock returns the same capability fields with `native-runtime-unavailable` and never fabricates success evidence.

This avoids turning Utility into another delivery value or overloading Role binding semantics.

### 7. Additive frontend service contract

The shared Skill response gains a delegation capability object containing support state and safe reason. The Tauri adapter maps native responses; the Web adapter deterministically reports unavailable. Settings displays status only in this change; invocation remains Agent-driven through the fixed native tool, so no arbitrary “run Utility” UI is introduced.

## Risks / Trade-offs

- **Provider child generations can increase cost and latency** → Enforce strict host caps, expose duration/counts, and allow parent cancellation.
- **A Utility response may still be incorrect or unsafe** → Treat it as untrusted model output subject to the parent runtime's existing tool and approval policies.
- **Revision drift could weaken evidence attribution** → Capture revision during resolution and revalidate immediately before admission.
- **Terminal callbacks may race with cancellation or timeout** → Serialize state transitions and accept only the first valid terminal state.
- **Existing Utility packages may assume unsupported tools** → Advertise the restricted tool set and fail with a safe unsupported-capability classification.
- **Cross-context coupling may grow** → Exchange narrow DTOs/ports through context APIs; architecture tests forbid infrastructure backchannels.

## Migration Plan

1. Add contracts and runtime-aware capability reporting while leaving delegation disabled.
2. Add resolution/admission state machine and tests behind native API runtime wiring.
3. Add restricted child execution and cancellation propagation.
4. Enable the fixed native tool for supported Agents.
5. Connect safe lifecycle projections to observability and evolution evidence.
6. Update frontend adapters/UI and remove the blanket Utility-unavailable assumption.

Rollback disables tool registration and reports Utility delegation unavailable. Existing Skill packages, bindings, and evidence remain readable; no destructive data migration is required.
