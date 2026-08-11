## Context

See `proposal.md` for motivation and the delta specifications for behavior. The permissions domain already persists `parent_principal_id` and `budget_config` but deliberately rejects non-null parent relationships. Execution observability can describe delegated topology, while the native API-agent runtime already owns provider-format translation, multi-turn tool execution, cancellation, approvals, messages, and unified permission evaluation.

This change assumes `establish-effective-skill-runtime` and `add-skill-overlay-governance` are implemented. Delegation must capture the same canonical Utility identity, effective revision, trusted Overlay-applied instructions, logical resources, assignment, pin state, and usage sidecars seen elsewhere. It must not create a second Agent runtime or permission engine.

## Goals / Non-Goals

**Goals:**

- Activate Utility Skills as bounded child-Agent capabilities for OnePiece and custom native API Agents.
- Reuse the existing provider, tool, permission, approval, cancellation, persistence, and observability infrastructure.
- Keep child context and tools narrower than the parent by construction.
- Make delegation attempts visible, cancellable, durable, and suitable as later self-evolution evidence.
- Preserve a stable provider tool catalog as Utility inventory changes.
- Keep desktop and Web/mock frontend contracts behaviorally aligned.

**Non-Goals:**

- Recursive, parallel, or autonomous multi-level Agent trees.
- Adding native delegation commands to third-party CLI protocols.
- Running `scripts/` content, dynamically importing functions, or registering one provider tool per Skill.
- Giving a child the complete parent transcript, hidden reasoning, or unrestricted memory automatically.
- Selecting a different provider or model for each Utility; Skill configuration management may add that later.
- Allowing a Utility to bypass parent permission mode, policy, approval, workspace, or sandbox boundaries.
- Turning Utility output into an independent chat speaker or editable conversation branch.

## Decisions

### 1. Add one fixed `delegate_skill` tool

The native provider-agnostic tool catalog gains a fixed definition:

```text
delegate_skill
├─ skill_id: canonical id or alias
├─ task: bounded string
├─ context_summary?: bounded string
└─ resource_uris?: bounded logical skill:// URI list
```

The runtime resolves current eligibility when dispatching the call. The provider schema does not enumerate installed Utilities, aliases, tool capabilities, or limits. Agents discover metadata through `list_skills`; delegation returns the canonical id.

This preserves provider prompt/tool cache stability, bounds the catalog, and keeps permission decisions in the runtime rather than encoded in generated schemas.

Alternatives considered:

- Register one provider tool per Utility. Rejected because inventory changes alter schemas, names can collide, and thousands of Skills would create an unbounded catalog.
- Add a generic `spawn_agent` tool. Rejected because it does not enforce Utility assignment, effective instructions, declared capabilities, or stable governance identity.
- Inject Utility instructions into the parent system prompt. Rejected because it collapses the Role/Utility boundary and increases context without delegation.

### 2. Extend Utility metadata with a declarative delegation contract

`SKILL.md` may declare:

```yaml
type: utility
delegation:
  tools:
    - file-read
    - content-search
    - filename-search
  max_rounds: 6
  timeout_seconds: 90
  max_context_chars: 12000
  max_output_chars: 8000
```

Capability ids are VaneHub-owned stable categories mapped to existing tool operations, never executable entry points. Missing `delegation` means the default read-only set. Unknown ids make the Utility delegation-unavailable. Declared limits can only lower platform ceilings.

Initial platform ceilings are centralized policy constants rather than user settings:

```text
delegation depth                 1
active child per parent run     1
attempts per parent generation  4
child model/tool rounds         8
duration                        120 seconds
task                            8,000 characters
explicit context                16,000 characters
final summary                   12,000 characters
evidence references             20
```

Existing per-tool input/output limits remain unchanged and apply inside these aggregate ceilings. The constants are serialized into attempt records so results are explainable if defaults later change.

Alternatives considered:

- Let Skill instructions name arbitrary tools. Rejected because free text is not an authorization contract.
- Require every Utility to declare all limits. Rejected because a secure read-only default supports simple content-only Utility packages.
- Allow Utility metadata to raise limits. Rejected because package authors must not control platform resource ceilings.

### 3. Resolve and freeze an effective Utility snapshot before approval

Dispatch resolution requires:

- native API parent with delegation capability;
- canonical id or unambiguous alias;
- effective `type: utility` definition for the active canonical workspace;
- enabled state and assignment to the parent stable Agent id;
- trusted package and Overlay chain;
- deterministic effective instructions and resources;
- valid delegation metadata and capability ids.

The runtime creates a prepared snapshot containing canonical id, effective package and Overlay revision hashes, instructions, permitted logical resources, declared and effective capabilities, captured limits, workspace, and parent generation. The approval resource includes these hashes. After approval, the runtime rechecks current identity, trust, assignment, pin and revision; any change makes the approval stale and requires a new call.

Role Skills, untrusted imports, conflicted Overlays, disabled or unassigned Utilities, and unsupported runtimes fail before child creation. A refusal is persisted as parent tool outcome but does not increment Utility use.

Alternatives considered:

- Resolve the Utility after approval. Rejected because the user could approve one revision and execute another.
- Continue with the approved old revision if the effective winner changes. Rejected because stale package content may no longer be trusted or assigned.

### 4. Model delegation as an attempt aggregate inside the native Agent runtime

The application model is:

```text
UtilityDelegationAttempt
├─ attempt_id
├─ parent_run_id / parent_generation_id / parent_message_id
├─ parent_agent_id / parent_principal_id
├─ child_principal_id
├─ canonical_skill_id / effective_revision
├─ canonical_workspace?
├─ provider_profile_snapshot / model_snapshot / interface_format
├─ declared_capabilities / effective_capabilities
├─ effective_limits
├─ state
├─ started_at / terminal_at
├─ tool_count / approval_count / model_rounds
├─ execution_run_id / span links
└─ terminal_result?
```

States are `prepared`, `awaiting_approval`, `running`, `awaiting_child_approval`, `completed`, `denied`, `failed`, `cancelled`, `timed_out`, `limit_exceeded`, and `interrupted`. Transitions are monotonic and terminal states cannot resume.

The record is persisted before the first model request. Starting the first provider request and incrementing Utility `use_count` occur under one application transaction boundary, so denied or pre-start-cancelled work does not count as use.

The child generation reuses the native provider adapter and tool loop through a `GenerationOwner` abstraction that distinguishes parent chat generation from Utility child attempt. This avoids recursive calls back through Tauri commands and lets cancellation, approvals, messages, and observability attach to the right owner.

Alternatives considered:

- Create a normal user-visible session per delegation. Rejected because it pollutes session navigation and gives a child more conversation semantics than intended.
- Run a fire-and-forget background task. Rejected because the parent needs a deterministic tool result and cancellation ownership.
- Reuse the parent generation id for the child. Rejected because approvals, attempts, metrics, and cancellation would become ambiguous.

### 5. Capture the parent's provider and model, not its hidden state

At attempt creation, the child captures the parent's provider profile id, interface format, model id, reasoning options allowed for child use, and an opaque credential handle. Runtime credential resolution occurs inside the provider adapter; credentials are never serialized into attempt prompts, results, logs, or frontend contracts.

The running child does not change when the user later changes provider or model selection. If the captured profile, model, or credential becomes unavailable before the first request, the attempt fails; it never silently falls back. Removing credentials during an active attempt causes the next provider call to fail closed.

Compaction is not used to manufacture child context. The child has its own bounded model/tool round history retained only for the attempt and summarized into the terminal result.

Alternatives considered:

- Let Utility metadata choose a model. Rejected for this phase because it introduces Skill configuration, credential selection, and cost policy together with delegation.
- Share the parent provider conversation id. Rejected because provider-side state could leak the full parent transcript and confuse independent tool loops.

### 6. Build an explicit minimal child prompt envelope

The child system input contains:

1. native child safety and result-format instructions;
2. trusted effective Utility instructions;
3. canonical Utility and workspace metadata without host paths;
4. effective tool capability and limit declaration;
5. permitted logical Utility resource index.

The child user input contains the delegated task and optional explicit context summary. Logical resource URIs are validated against the Utility snapshot before inclusion. The full parent transcript, hidden reasoning, unrelated memories, environment, credential data, and arbitrary workspace files are not copied automatically.

Task and context overflow is rejected rather than truncated because silent truncation could change intent. Child tool and model output follows existing bounded result behavior. Context text is treated as untrusted data and delimited from Utility instructions.

Alternatives considered:

- Include the last N parent messages. Rejected because message count does not provide a meaningful privacy or token boundary and can leak unrelated data.
- Let the child read the parent message database. Rejected because it bypasses explicit context selection.
- Automatically attach all Skill resources. Rejected because progressive disclosure and logical resource permissions should remain in force.

### 7. Compute the child tool catalog as an intersection

The effective child catalog is:

```text
platform child allowlist
∩ parent permission-mode ceiling
∩ Utility declared capability map
∩ effective trust and availability
∩ runtime feature availability
```

The platform allowlist initially supports bounded file read/write, scoped edit, content search, filename search, memory read/write where already available, and fixed Skill reads. Shell may be enabled only when explicitly declared and parent mode permits it. MCP and `delegate_skill` are excluded. Scripts bundled with the Skill remain inert data.

The same intersection is enforced twice: catalog construction and dispatch. Plan mode accepts delegation only if the effective capability set is wholly Plan-compatible and read-only. It does not merely remove a mutating tool from a Utility that declared it, because doing so could make the Utility's behavior materially different from what the parent selected.

Only one child is active per parent generation. Existing provider responses containing multiple delegation calls are processed in deterministic tool-call order; calls after the first active child return a concurrency-limit result rather than queueing hidden work.

Alternatives considered:

- Give the child the parent's full tool catalog and rely on approvals. Rejected because absence from the catalog is a stronger least-privilege boundary.
- Silently narrow a mutating Utility in Plan mode. Rejected because the child instructions may assume unavailable side effects and produce misleading success.
- Permit recursive delegation with a depth counter. Rejected because permissions, context, costs, and cancellation need operational evidence at depth one first.

### 8. Activate stable child principals with a parent-chain ceiling

The existing principal table is used. A stable child principal is keyed by `(parent_agent_principal_id, canonical_utility_id)` and stores the parent id plus serialized hard budget configuration. It is reused across attempts; attempts remain unique.

Only an internal delegation service capability can create or update non-null parent relationships. Validation requires an Agent parent, depth exactly one, no cycle, and no parent mutation after creation. Existing root principals remain unchanged.

Permission evaluation for a child action combines the child and ancestor chain:

1. evaluate applicable explicit Deny policies and grants across child and parent; any Deny wins;
2. evaluate the child principal's own grants and policy;
3. evaluate the parent as a ceiling: parent Ask prevents child Allow from becoming automatic, and parent Allow does not grant a child action the child itself has not been allowed;
4. final result is Allow only when child and parent chain all resolve Allow, Deny if any resolve Deny, otherwise Ask;
5. append one audit record with child principal, parent chain, channel, action, resource, risk, and deciding mechanism.

New child principals use the Standard policy template by default. Remembered child-action grants attach to the stable child principal and existing Once/Session/Project/Global scope behavior. A delegation-start grant is a separate `agent.delegate` action on a revision- and capability-bound Utility resource and cannot authorize child tools.

Alternatives considered:

- Execute every child action as the parent principal. Rejected because users could not distinguish or constrain a Utility independently.
- Let parent Allow automatically flow down. Rejected because delegation must not amplify trust.
- Create an ephemeral principal per attempt. Rejected because policy settings and remembered decisions would become unusable and audit identity unstable.

### 9. Use two permission gates without duplicating engines

Delegation start is evaluated through the unified permission service before model execution. Its resource includes canonical Utility id, effective revision, parent Agent, workspace scope, capability hash, and risk tier. Default is Ask. The existing approval broker owns pending state, event/pull reconciliation, timeouts, remembered scopes, and audit.

Once running, every child tool call independently evaluates under the child principal chain. Start approval means “allow this Utility attempt to begin,” not “approve its future actions.” The approval UI displays parent, Utility, task summary, capability ceiling, and action context.

Pending approvals contain an owner reference: parent generation for start approval, child attempt for child-action approval, plus parent generation for cascading cancellation. Cancelling a child stales its child approvals; cancelling the parent stales both start and child approvals.

Alternatives considered:

- Treat assignment as permanent start approval. Rejected because assignment controls availability, not whether a specific task and capability revision should run.
- Approve delegation only once and auto-approve every child action. Rejected because the actual resources and commands are unknown at start time.
- Build a Utility-specific approval queue. Rejected because pending approvals already have durable UI and fail-closed semantics.

### 10. Enforce hierarchical cancellation and deterministic terminal results

Each parent generation owns a cancellation token; each child attempt receives a child token. Parent cancellation propagates to the child provider stream, tool execution, and approval waits. Child cancellation does not cancel the parent. Duration and round limits cancel the same token with a different terminal reason.

The parent `delegate_skill` tool call waits for one terminal child result while forwarding bounded lifecycle events to chat and observability. All terminal paths return:

```text
DelegationResult
├─ attempt_id
├─ canonical_skill_id / effective_revision
├─ status
├─ summary
├─ evidence_refs[]
├─ effective_limits
├─ model_rounds / tool_count / approval_count
├─ duration_ms
├─ truncated
└─ safe_error?
```

The result excludes hidden reasoning and raw transcript. A child failure is a tool result, not an automatic parent-generation failure; the parent model decides how to continue. Parent cancellation is the exception and stops the parent loop.

On application restart, records in non-terminal states become `interrupted`. Provider or tool calls are not automatically replayed because external side effects may already have occurred. Pending approvals become stale through existing recovery semantics.

Alternatives considered:

- Retry child provider failures automatically. Rejected because retries consume budget and can duplicate tool side effects; a later explicit delegation creates a new attempt.
- Store only a final text response. Rejected because governance, UI, evidence, and self-evolution need structured status and execution links.

### 11. Persist attempts without copying unbounded content

SQLite adds `utility_delegation_attempts` plus normalized links to existing execution runs, tool-use records, permission audit, parent messages, sessions, and Skill identity. Attempt rows store status, safe hashes and ids, captured configuration, limits, counts, timestamps, bounded result summary, and safe error. Raw Utility instructions, credentials, hidden reasoning, full task/context, and unrestricted paths are excluded.

The existing completed parent message stores bounded delegation activity references and summary projections. History queries join by canonical Utility id, workspace, parent Agent, status, and time with cursor pagination. Project isolation uses canonical workspace identity.

Utility `use_count` increments once at the transition to running, immediately before the first child provider request. Persistence and the counter update share a transaction; if the provider request then fails, the count remains valid because execution genuinely began.

Alternatives considered:

- Persist full child transcripts for debugging. Rejected because privacy, size, and hidden-reasoning boundaries outweigh convenience.
- Store attempts only inside message JSON. Rejected because Skill history, status queries, recovery, and observability need indexed durable records.

### 12. Extend observability rather than creating delegation logs

The parent tool span links a child delegation span/run carrying attempt id, stable parent Agent, child principal, canonical Utility, effective revision hash, workspace hash, capability ids, limits, fidelity, and state transitions. Child model rounds, tool calls, approvals, cancellations, and terminal result are correlated through existing execution topology.

Default telemetry is metadata-only. Task/context text, Skill instructions, model reasoning, credentials, file contents, raw commands, and full paths never become span attributes or metric dimensions. Low-cardinality metrics cover Utility id, terminal status, duration buckets, limit reason, approval outcome, and tool counts under existing retention.

Unified logging receives operational failures through the central service with the same redaction. No per-Utility or per-delegation log files are introduced.

Alternatives considered:

- Use history records as the only diagnostics. Rejected because cross-runtime tracing already correlates parent, tools, permissions, and providers.
- Add task text as a trace attribute. Rejected because task content can contain source code and secrets.

### 13. Present child activity inside the parent chat message

Streaming adds typed delegation lifecycle events keyed by attempt id. The frontend reduces them into one collapsible child activity on the parent assistant message:

- awaiting approval;
- running with elapsed time;
- bounded tool and approval activity;
- terminal status, summary, evidence, limits, and truncation;
- independent cancel action while active.

It is not rendered as a speaker message. Reloaded history reconstructs the activity from persisted projections, including `interrupted` recovery. The existing parent stop action cascades; the child cancel calls a new service method scoped to attempt id.

`agent-service.ts` owns the shared contracts. `tauri-agent-client.ts` maps native events and commands. `web-agent-client.ts` runs deterministic state machines for eligible, approval, running, child approval, completed, denied, failed, limited, cancelled, and interrupted scenarios with no provider or filesystem effects.

Alternatives considered:

- Open a separate child session tab automatically. Rejected because transient utility work would overwhelm session navigation.
- Hide delegated work inside a generic tool card. Rejected because users need child cancellation, approvals, status, limits, and evidence visibility.

### 14. Extend Skill settings without mixing Role and Utility relationships

Utility cards and details show declared/effective capability ids, requested/effective limits, trust, revision, availability, use count, last use, supported Agent assignments, and history. The selected-Agent board labels API Utility relationships as delegated capability, Role API relationships as prompt/load behavior, and CLI Role relationships as mount behavior.

Unsupported CLI Agents do not show an Assign action for Utility delegation. Existing unsupported associations remain visible as repair state. History is paginated and links to execution timeline and permission audit views through typed service calls.

React components remain runtime-neutral, use Tailwind and existing component patterns, and are split below the 300-line production-file limit. Approval presentation is enhanced through the existing pending-approval UI, not a Skill-specific queue.

Alternatives considered:

- Show Utilities as normal API prompt bindings. Rejected because it would imply eager or on-demand Role loading instead of child execution.
- Hide unsupported CLI relationships. Rejected because silent records are difficult to repair and misrepresent actual state.

## Risks / Trade-offs

- [Two approval levels feel repetitive] → Clearly distinguish start authority from concrete child actions and support existing scoped remembered grants without allowing one to imply the other.
- [Stable child principals accumulate] → Reuse one per parent Agent and canonical Utility, expose them through existing policy administration, and archive only after assignments, grants, and audit retention allow it.
- [Child output is too terse for the parent] → Provide bounded summaries and evidence references while keeping full hidden reasoning and transcripts out of the contract.
- [Provider costs increase unexpectedly] → Default start to Ask, show model and limits, cap attempts/rounds/duration, and persist measurable usage metadata.
- [Plan mode is weakened by delegation] → Accept only Utilities whose entire effective contract is read-only and enforce the Plan ceiling at catalog and dispatch.
- [A Utility writes files concurrently with the parent] → Allow one child at a time, block the parent tool call until terminal result, and keep every write behind the existing sandbox and permission pipeline.
- [A package changes after approval] → Bind approval to effective revision and capability hashes and recheck immediately before child creation.
- [Permission inheritance becomes hard to reason about] → Use explicit Deny first, require all chain members to allow for automatic execution, and record the deciding principal and mechanism.
- [CLI users expect the same native tool] → Show delegation as unsupported for CLI Agents until a specific adapter can make authenticated native calls; do not claim parity by mounting Utility instructions.
- [Runtime restart leaves ambiguous side effects] → Mark attempts interrupted and never auto-retry model or tool calls.

## Migration Plan

1. Complete and validate effective Skill runtime and Overlay governance prerequisites.
2. Add Utility delegation metadata parsing, capability mapping, eligibility, and unavailable-reason tests while Utilities remain execution-disabled.
3. Add delegation attempt persistence, stable child-principal creation, graph validation, and parent-chain permission evaluation behind a feature gate.
4. Refactor the native generation and tool loop around parent/child execution ownership without changing existing parent behavior.
5. Add fixed `delegate_skill` schema, prepared snapshots, start approval, child prompt construction, tool intersection, limits, and structured results.
6. Add hierarchical cancellation, restart recovery, usage counting, observability, unified logging, and history queries.
7. Enable read-only Utility delegation first and verify Plan and Standard mode boundaries.
8. Enable declared mutating capabilities behind start and child-action approvals after sandbox and policy regression tests pass.
9. Add shared frontend contracts, Tauri and Web/mock adapters, chat child activity, approval context, Skills UI, localization, and accessibility tests.
10. Run the repository's full validation suite and strict OpenSpec checks before removing the feature gate.

Rollback disables new delegation dispatch and removes `delegate_skill` from provider catalogs. Running child attempts are cancelled and persisted terminally before downgrade. Attempt, principal, permission audit, message projection, usage, and observability records remain additive and readable or ignorable; rollback never rewrites Skill or Overlay content. Existing root Agent principals and policies are unchanged. Re-enabling verifies principal graphs and marks any leftover non-terminal attempts interrupted before accepting new work.

