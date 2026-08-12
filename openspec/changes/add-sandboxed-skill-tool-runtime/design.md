## Context

See `proposal.md` for motivation. Native API agents currently assemble a fixed built-in tool catalog plus runtime-discovered MCP tools, then dispatch calls through a central execution loop and unified permission service. Effective Skill loading and Overlay governance are planned separately, but there is no trusted executable extension boundary for a Skill package and no embedded script engine dependency.

The design must preserve four properties: external CLI sessions cannot be assumed to understand local tools; React stays behind `AgentService`; Overlay content remains non-executable; and all native logs use unified, redacted logging.

## Goals / Non-Goals

**Goals:**

- Let an effective Skill revision contribute schema-defined tools to eligible native API agent contexts.
- Support common wrappers without code and bounded custom computation when a module is necessary.
- Make trust, capability, permission, lifecycle, limits, provenance, and rollback explicit and testable.
- Contain malformed or hostile tools to their own immutable revision and preserve cancellation semantics.

**Non-Goals:**

- Executing Python, JavaScript, shell, batch, native binaries, or dynamic libraries shipped by a Skill.
- Providing full WASI, arbitrary filesystem/network/process access, or raw environment and secret access.
- Injecting local tools into third-party CLI processes that do not implement a supported bridge.
- Allowing self-evolution or Overlay mutations to change executable tool content.
- Building a general plugin runtime or a graphical tool-code editor.

## Decisions

### 1. Use a versioned manifest with two implementation kinds

Each Skill may contain `scripts/tools.json` and integrity-bound files under `scripts/modules/`. The manifest has a version, Skill revision witness, and entries with local id, schemas, implementation kind, capabilities, limits, and hashes. Canonical runtime names use `skill__<encoded-skill-id>__<encoded-tool-id>__<short-revision>`; the model-visible name maps back to an immutable internal key rather than being parsed as authority.

`declarative` entries describe a bounded call template targeting an existing registered operation. Templates support schema-bound field projection and constants, but no loops, conditionals, shell expansion, arbitrary expressions, or free-form pipelines in the first release. `wasm` entries point to a content-hashed module and export.

This covers most utility adapters without adding code while preserving an escape hatch for pure custom logic. A subprocess JSON-RPC design was rejected because cross-platform containment of host runtimes is weak. An embedded general scripting language was rejected because its host API would become a second permission system.

### 2. WebAssembly is optional and capability-based

Implementation introduces a `SkillToolModuleRuntime` port. The native adapter uses a pinned, supply-chain-reviewed WebAssembly engine configured without inherited WASI. The module initially receives only bounded input and result buffers. A single structured host-call import can request an allowed existing operation; the host checks the manifest capability, recursion/call limits, execution mode, and unified permission service before dispatch.

Default ceilings are centrally configured and can only be tightened by manifests: 10-second wall time, deterministic fuel budget, 64 MiB linear memory, 1 MiB input, 1 MiB output, 8 host calls, delegation depth 4, and bounded per-Skill concurrency. Exact fuel is calibrated in implementation tests because engine versions differ. Epoch interruption and the generation cancellation token terminate work. No module instance is reused across trust revisions; pooling is permitted only for immutable compiled artifacts with fresh stores.

WASM components and raw core modules were both considered. The first implementation uses the smallest stable engine API that supports typed JSON byte buffers, fuel, memory limits, and interruption; the application port prevents that choice from leaking into domain contracts.

### 3. Trust is bound to content; authorization remains separate

Trust records bind Skill id, source scope, base revision hash, manifest hash, implementation hashes, capability digest, decision, actor, and timestamp. System-bundled modules may enter with distribution trust; Project, User, and Remote revisions require explicit trust unless an enterprise policy establishes a stronger signed-source rule. Any bound value change invalidates trust.

The trust gate runs before instantiation. Each host call then uses a principal containing parent agent, Skill/tool identity, revision, workspace/session, and delegation chain. Requested capabilities are an upper bound, never permission. Existing policy returns Allow, Ask, or Deny. Approval requests include provenance and an immutable request witness. This avoids creating a parallel authorization path.

### 4. Tool availability is contextual, not installation-global

An effective registry snapshot is assembled per generation context:

- Role Skill tools are eligible only while that Role revision is loaded in the session.
- Utility Skill tools are eligible only inside the delegated child execution.
- Shadowed, disabled, archived, invalid, untrusted, or quarantined revisions contribute no tools.
- Plan-mode and other execution-mode restrictions are intersected with Skill capabilities.
- External CLI sessions receive no local Skill tools unless a future explicit bridge capability exists.

Refresh builds and validates a complete replacement snapshot, then swaps it atomically. In-flight calls hold an `Arc`-style immutable snapshot; new calls see the replacement. Canonical names include a revision fragment, so stale model calls fail instead of being silently rebound.

### 5. Keep the runtime behind domain ports and service adapters

Rust adds a `skill_tools` context with domain manifest/trust/lifecycle models, application services, and infrastructure adapters for filesystem discovery, SQLite state, WebAssembly compilation/execution, schema validation, unified logs, and the existing agent tool/permission gateways. The agent runtime receives catalog and execution ports rather than depending on Skill filesystem details.

Tauri commands expose list, validate, trust/revoke, enable/disable, quarantine/recover, and diagnostics operations as `Result<T, String>` or mapped domain errors. TypeScript contracts extend `AgentService`; `tauri-agent-client.ts` is the only frontend layer that invokes commands. `web-agent-client.ts` returns the same shapes with explicit unsupported native execution state.

### 6. Overlay and self-evolution cannot mutate executable content

Executable paths and manifests are reserved base-package paths. Overlay validation rejects them before transaction creation, and effective-content assembly never applies Overlay operations to those paths. Evolution candidates may recommend a separately reviewed package revision, but auto-apply and Curator approval cannot convert that recommendation into executable content. This is a hard boundary rather than a risk-score threshold.

### 7. Validate at install, refresh, trust, and call time

Validation is layered: path containment and file-size checks; manifest schema and id normalization; hash verification; duplicate/collision checks; JSON Schema complexity limits; declarative target/capability checks; WebAssembly format, import/export, memory, and feature checks; then compilation under runtime limits. Trust is refused when validation is not clean.

Input and output validation occurs on every call even after static validation. Repeated deterministic failures increment a per-revision circuit breaker and can quarantine that tool. Operator recovery requires a clean revalidation and an explicit action; content changes create a new revision instead of clearing old evidence.

### 8. Observability stores evidence, not sensitive payloads

Unified logs record correlation ids, identities, hashes, capability and permission outcomes, timing, resource counters, lifecycle, and error codes. Raw input/output is not persisted by default; schema-declared sensitive fields, credentials, paths, and command content are redacted before any diagnostic summary. Transcript entries expose tool provenance and bounded results according to existing message persistence rules. Successful calls update Skill use counts.

### 9. UI is an inspection and governance surface

The existing Skill detail view gains a Tools tab with inventory and status summary, capability list, integrity/trust revision card, validation report, recent bounded outcomes, quarantine reason, and explicit governance actions. Trust uses a confirmation dialog showing source, hashes, capability diff, and validation result. Enablement is separate from trust. Unsupported Web execution is visible as a capability state, not an error toast.

Components are split below the 300-line limit and use existing Tailwind patterns, accessible labels, focus management, status text, and keyboard actions. No raw module editor or arbitrary execution console is provided.

## Risks / Trade-offs

- [A WebAssembly engine increases binary size, compile time, and dependency surface] → Keep it behind an optional adapter, pin versions, review advisories/licenses, and retain declarative-only operation when unavailable.
- [A host-call bridge could become a capability escape hatch] → Use one structured gateway, exact manifest allowlists, unified permission evaluation, cycle detection, and conservative depth/call ceilings.
- [Provider tool-name length or character limits may differ] → Generate bounded encoded names, retain immutable internal ids, validate collisions, and test every provider translation.
- [Schema validation can consume excessive resources] → Limit schema size/depth/keywords, precompile validators, cap payloads, and reject unsupported recursive constructs.
- [Atomic refresh retains old compiled artifacts briefly] → Hold them only while referenced by in-flight calls, cancel on security quarantine, and zeroize transient buffers where applicable.
- [Trust prompts can train users to approve blindly] → Show capability diffs and provenance, separate trust from permissions, and require a new decision only when integrity-bound content changes.
- [CLI users may expect universal availability] → Label Skill tools as native API-agent capability and report unsupported bridging honestly.

## Migration Plan

1. Add persistence tables and read-only manifest discovery with the runtime feature disabled.
2. Add validation, integrity, trust, and UI inspection; existing Skills without manifests remain unchanged.
3. Enable declarative tools for trusted test Skills and integrate catalog, permissions, transcript, cancellation, and unified logs.
4. Add the pinned WebAssembly adapter behind a feature/config gate and pass adversarial resource-limit tests.
5. Enable per-revision governance actions and quarantine recovery, then expand rollout from system test Skills to explicitly trusted local Skills.
6. Rollback by disabling Skill tool execution globally and atomically removing contributed tools; retain trust, validation, audit, and usage records for diagnosis. No SKILL.md migration is required.
