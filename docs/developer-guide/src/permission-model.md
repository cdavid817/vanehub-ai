# Permission model

Every gated action — whether requested by a native API Agent's tool-use loop or forwarded through the Claude Code permission-hook bridge — is evaluated through a single decision point. There is no separate decision engine for CLI-originated calls.

## Unified decision model

Evaluation resolves a `(principal, action, resource)` triple to exactly one of `Allow`, `Deny`, or `Ask`. A principal is identified by a **stable agent id alone** — one durable principal per Agent, persisting across every session that Agent participates in. Session id and generation id are per-evaluation context, not part of the principal's identity. So an Agent participates in a new session using the same principal and policy assignment as its other sessions, not a session-scoped one.

Unmatched actions (no policy matches the principal/action/resource) resolve to `Ask`, never `Allow`.

## Resolution order: explicit-Deny-first

Conflicting policy matches resolve with explicit `Deny` priority over explicit `Allow`, and explicit `Allow` priority over the default `Ask`.

## Remembered grants: canonical identity and precedence

A remembered decision is a **value of a canonical key**, not a row appended to a list. The key is `principal + action + resource + scope + the scope's owner`, where the owner is the session id, the project key, or the global sentinel. Remembering the same decision again is an update: the effect is replaced and the revision advances. Three scope-specific partial unique indexes make that physical, so a future writer cannot reintroduce a second row for one key.

Selection is decided by the database in one ranked query, not by the caller:

| Rank | Match |
| --- | --- |
| 3 | Session row whose session equals the evaluation session |
| 2 | Project row whose project equals the evaluation project |
| 1 | Global row |

A more specific scope deliberately overrides a broader one, **including a broader `Deny`** — the narrower row is the later, more informed statement about a narrower situation. Specificity is evaluated before effect; folding "deny always wins" into the ranking is how order-dependence would return.

`Once` and `Ask` are unrepresentable as remembered grants rather than checked for: `RememberedScope::parse` refuses the first and `PersistedEffect::parse` the second, so a new persistence path cannot forget the rule. There is no wildcard, prefix, or path-normalisation matching.

## Approval broker

Pending approval requests are held in the native runtime as the single source of truth, independent of whether any frontend event about them was received. A missed frontend event cannot leave a generation silently waiting: the frontend pushes new pending approvals via events **and** reconciles by pulling the full pending list on mount/reconnect. A pending approval is resolved with both an approve/deny decision and a memory scope of `Once`, `Session`, `Project`, or `Global`.

Each pending entry carries a phase — `Pending`, `Resolving`, or `Committed`. Claiming is atomic and single-winner, so two callers submitting opposite decisions at the same moment cannot both proceed: one gets the request, the other gets the winner's resolution id and reports that result. A claim can be reverted only by its holder, and only before anything durable was written. After commit it is never returned to `Pending`, because offering the request again would invite a second decision for one that already has an answer.

## Commit before effect

The decision and its effect happen in two different places — a row in SQLite, and a native Agent or an HTTP waiter being released — and those cannot be made atomic with each other. So the ordering is stated instead:

```text
claim → reserve → commit → deliver → acknowledge → activate
```

Each step is chosen for what it makes impossible:

- **reserve** proves the originating waiter and generation are still current *without* resuming them, so a stale generation is discovered before anything durable exists. `agent_runtime` publishes one boolean for this; it never hands out a generation handle.
- **commit** is one transaction writing the immutable resolution, its decision audit, and any remembered-grant intent together. `Allow` therefore cannot reach anyone before its evidence exists.
- **acknowledge** is what activates the remembered grant. A grant written by the commit is `pending_delivery` and invisible to evaluation until the waiter confirms it applied the decision — so an approval that never actually arrived cannot authorize the *next* attempt.

A retry carries the same immutable `resolution_id`, and the receiving waiter applies a resolution at most once. Delivery outcomes are typed rather than boolean: `delivered`, `stale`, `delivery_failed`, `resolving`, `already_resolved`, `not_found`. Only the first means the tool ran; `delivery_failed` means the decision is durable and reached nobody, and the UI must not offer a second decision for it.

The timeout sweep reports which requests expired and feeds those ids back through the same use case a human decision uses. It has no delivery shortcut of its own, so a timeout arriving while somebody is clicking loses the claim rather than writing a competing resolution.

## Restart and storage-failure semantics

At startup, every resolution still in `committed` or `delivery_failed` had a waiter in a process that no longer exists. It is marked `aborted_by_restart` and becomes durable evidence and nothing else: no pending request is recreated, no effect is delivered to a new generation, and its grant stays inactive. A crash after the waiter applied the effect but before the acknowledgement was recorded lands here too — least privilege is chosen over guessing that delivery happened.

Evaluation continues to fail closed, and now leaves attributed evidence: a storage failure is audited under an `evaluation_error` decider with a stable reason code. If the audit store is unavailable too, one redacted line goes through unified logging carrying only the action token, the reason code, and the session and generation ids. The resource, the tool input, and the underlying error text are all deliberately absent — the first two are user content and the last can quote a query.

## CLI launch-flag projection

For `gemini-cli`, `codex-cli`, and `opencode`, an Agent principal's assigned policy template (`readonly`, `standard`, `trusted`, or `yolo`) is projected into that tool's own native approval/sandbox launch parameters whenever its Agent Terminal starts interactively. Only catalog-legal, non-bypass parameter values are used — no raw bypass flag (e.g. one whose name contains "dangerously") is introduced to reach a template's behavior. `trusted` and `yolo` project to the same launch parameters.

## Claude Code permission-hook bridge

`PreToolUse` requests from the hook wrapper are translated to an `Action`/`Resource` pair and resolved through the same `evaluate()`/`ApprovalBroker` pipeline as native API Agents. The hook matches only `Bash`, `Edit`, `Write`, `Read`, `Glob`, `Grep`, and MCP tool names (`mcp__*`), mapping them to `shell.exec`/`file.write`/`file.read`/`mcp.tool`; any other tool (e.g. `WebFetch`) is not intercepted and Claude Code's native behavior is unaffected. An `Ask` resolution creates a pending approval in the existing `ApprovalCard` UI and holds the HTTP response until a human decision or the timeout sweep.

## Where the design lives

This chapter orients contributors. The authoritative requirements live in the specs.

- [openspec/specs/permissions-core](../../../openspec/specs/permissions-core/spec.md) — the unified decision model and resolution order.
- [openspec/specs/permissions-approval](../../../openspec/specs/permissions-approval/spec.md) — the approval broker, pending state, and memory scopes.
- [openspec/specs/cli-agent-permission-launch-flags](../../../openspec/specs/cli-agent-permission-launch-flags/spec.md) — CLI launch-flag projection.
- [openspec/specs/claude-code-permission-hook](../../../openspec/specs/claude-code-permission-hook/spec.md) — the Claude Code `PreToolUse` bridge.

Permission evaluation lives in the `agent_runtime` bounded context; see [Native bounded contexts](native-contexts.md).
