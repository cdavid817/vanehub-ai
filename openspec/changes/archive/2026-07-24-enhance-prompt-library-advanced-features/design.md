## Context

The existing `tooling::prompt_hooks` bounded context owns built-in and user Hook manifests, two literal placeholders (`{{agentId}}` and `{{sampleInput}}`), in-place user Hook updates, safe assembly traces, and SQLite persistence. `agent_runtime` consumes the published Prompt Hook API through an effective-prompt gateway before provider launch. React uses `AgentService`; the Tauri and Web/mock adapters implement the same management surface.

This change spans the Prompt Hook domain, persistence, agent-runtime completion path, frontend adapters, and settings UI. Prompt text is sensitive: version history necessarily stores user-authored templates, while evaluation rows and unified diagnostics must not duplicate rendered Prompt bodies or user input.

## Goals / Non-Goals

**Goals:**

- Provide a small, extensible, backend-owned template-variable catalog with stable snake-case names and compatibility aliases for existing templates.
- Separate editable drafts from immutable published versions and guarantee that live assembly uses only the selected published version.
- Preserve an append-only audit trail for publication and rollback.
- Attribute safe terminal outcome and elapsed-time observations to every published Hook version fired by one Agent invocation.
- Expose useful per-version aggregates through equivalent desktop and Web/mock service contracts and localized UI.
- Keep cross-context communication through the Prompt Hook published API and an agent-runtime-owned consuming port.

**Non-Goals:**

- Arbitrary expressions, conditionals, loops, environment-variable access, filesystem reads, commands, or scripts in templates.
- Collaborative draft editing, approvals, cloud synchronization, experiments, traffic splitting, or automatic version promotion.
- Semantic scoring of model output, user ratings, statistical significance claims, or cross-Prompt causal attribution.
- Persisting rendered Prompt content, user input, model output, or failure diagnostics in evaluation rows.
- User-managed versioning of backend-owned built-in Hooks.

## Decisions

1. Use a typed variable context and an allowlisted catalog.

   Canonical variables are `agent_id`, `agent_name`, `current_time`, `sample_input`, and `session_id`. The renderer also accepts existing `agentId` and `sampleInput` aliases so current templates continue to work. Values are plain text replacements only. The domain parser reports referenced unknown variables; draft saves may preserve them, but publish and live rendering reject them. One clock snapshot is used for the complete assembly, formatted as RFC 3339 UTC. Preview obtains values through the same context builder and injected clock.

   Alternative considered: a general-purpose template engine. It adds dependency, expression, and sandboxing complexity that the requested substitutions do not need.

2. Store draft state separately from immutable published snapshots.

   `prompt_hooks_user` remains the identity and mutable operational record (enabled state, bindings, current published version). New `prompt_hook_drafts` and `prompt_hook_versions` tables store draft content and immutable snapshots. Creation produces a draft with no live published version. Publishing validates the draft, appends the next monotonically increasing version, atomically selects it, and removes the matching draft. Existing rows are migrated into version snapshots and selected as published.

   Alternative considered: add a status column to mutable rows. That permits historical rows to be overwritten and makes rollback/audit invariants harder to enforce.

3. Rollback republishes history rather than moving the active pointer backward.

   Rollback validates the selected historical snapshot and appends identical content as the next version with `rollback_from_version`. This preserves monotonic versions and records the actual deployment sequence. An unrelated draft remains intact so rollback cannot silently discard work.

   Alternative considered: repoint `published_version` to an old row. That obscures when rollback occurred and allows one version number to represent multiple deployment periods.

4. Treat built-ins as read-only published catalog versions.

   Built-ins expose their catalog version and can receive evaluation observations, but draft, publish, and rollback commands reject built-in ids. VaneHub upgrades built-ins through catalog releases, preserving existing governance.

   Alternative considered: duplicate built-ins into the user version tables. This would blur catalog ownership and weaken immutable built-in guarantees.

5. Record one idempotent observation per invocation, Hook id, and version.

   Prompt assembly returns safe fired references (`hook_id`, `version`) to `agent_runtime`. At terminal completion, the runtime calls a consuming `PromptEvaluationGateway`, implemented by the Prompt Hook API adapter, with an invocation id, terminal outcome (`succeeded`, `failed`, or `cancelled`), elapsed milliseconds, agent id, and fired references. SQLite uses a composite uniqueness constraint so retries do not double-count.

   Success rate is `succeeded / (succeeded + failed)`; cancelled executions are shown separately and excluded because user cancellation is not evidence of Prompt quality. Latency aggregates include succeeded and failed executions but exclude cancelled executions. No raw error, Prompt, response, session id, or command data is stored.

   Alternative considered: derive metrics from existing traces or unified logs. Traces are written at assembly time and do not know the terminal result; logs are redacted operational evidence, not a transactional analytics store.

6. Query bounded pre-aggregated version summaries.

   The repository computes count, success/failure/cancelled counts, success rate, and average/minimum/maximum elapsed milliseconds grouped by immutable version. The command accepts one Hook id and returns bounded version history plus aggregates; the UI does not download raw observations.

   Alternative considered: expose every execution to React. This increases data volume and makes privacy/retention harder to govern.

7. Preserve service-boundary parity.

   `AgentService` gains variable-catalog, draft save, publish, version-history, rollback, and evaluation-summary methods. Only `tauri-agent-client.ts` invokes new commands. The Web/mock adapter maintains deterministic in-memory versions and observations with the same normalized shapes. React owns display and user intent only.

## Risks / Trade-offs

- [Template variables can expose runtime context] → Keep the catalog small, exclude environment/path/secrets, and treat values as inert text.
- [Migrated hooks may contain unknown placeholders] → Preserve existing published content and compatibility aliases; require validation only when creating a new publication.
- [One invocation fires multiple Hooks, so metrics are correlated] → Label results as operational evidence, not causal attribution, and avoid rankings that imply statistical causality.
- [Agent completion can race with retries or cancellation] → Use an invocation id and an idempotent composite key; accept exactly one terminal outcome per Hook version.
- [Version history grows indefinitely] → Store compact text snapshots and aggregated queries now; defer configurable retention because deleting rollback history would weaken audit guarantees.
- [Draft/publish adds UI complexity] → Keep lifecycle actions in a focused version panel and retain compact Hook cards for inventory scanning.

## Migration Plan

1. Add versioned SQLite migration tables and columns without deleting current Prompt Hook data.
2. Backfill every existing user Hook row into an immutable version snapshot using its current version and select that version as published.
3. Extend domain/application models and repository operations, then expose published API and commands.
4. Extend the agent-runtime gateway to carry fired version references and record idempotent terminal observations.
5. Add TypeScript contracts and implement Tauri/Web adapters before enabling UI actions.
6. Add localized draft/version/evaluation UI and focused tests.

Rollback of the application code leaves additive tables in place. Existing Hook rows and their previously active content remain readable; evaluation collection can be disabled without affecting prompt assembly.

## Open Questions

- Whether a future experiment feature should support explicit traffic allocation between two published versions.
- Whether later retention policy should compact old execution observations after persisting monthly aggregates.
