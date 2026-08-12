## Context

See `proposal.md` for motivation. Chat messages receive tool status snapshots through a shared `ChatStreamEvent` contract. The shared reducer and Web/mock adapter append every snapshot, while `ToolUseBlock` maps the resulting array one-to-one into equally prominent `<details>` cards. Stable tool ids and all data required for reconciliation already exist, so no native or transport change is needed.

## Goals / Non-Goals

**Goals:**

- Make one logical tool call occupy one record throughout its status lifecycle.
- Make approval requests, active work, and failure totals immediately scannable.
- Reduce successful and recoverable failure-history height while preserving complete inspectability and keyboard access.
- Let users collapse the entire activity region without losing status awareness or hiding an approval decision.
- Use the same rendering and reconciliation semantics in desktop and Web runtimes.

**Non-Goals:**

- Changing tool execution, permission policy, native events, persistence, or Agent behavior.
- Inferring shell success or deleting distinct tool-call evidence merely because commands match.
- Rendering arbitrary provider input as HTML or exposing additional sensitive data.
- Adding a new UI dependency or a separate OnePiece-only component.

## Decisions

### Reconcile only by stable tool-use id

A small pure helper will replace an existing array entry when ids match and append otherwise. The latest defined input/output fields win while prior defined data is retained if a status-only event omits it. Both shared stream reduction and Web/mock message updates will use this helper.

Grouping by name or command was rejected because separate legitimate calls can execute identical commands. Fixing only the renderer was rejected because duplicate snapshots would still pollute persisted Web state and downstream reporting.

### Separate actionable activities from terminal history

The activity container will derive ordered groups: approval-required, active (`pending`/`running`), failed, then completed. Approval-required and active groups render immediately. Failed and completed activities live behind separate summary disclosures, reducing height without losing evidence. Failure totals remain visible in the outer summary. The failure disclosure is open initially only when the containing assistant message has terminal `failed` status; an ordinary failed tool call is treated as recoverable diagnostic history.

Approval cards remain attached to their individual stable call ids and are never placed inside the default-collapsed completed history.

### Aggregate consecutive identical failure presentation

Distinct calls remain in message data. At render time, consecutive failures with the same tool name, safe input preview, and serialized output signature are represented by one row with an occurrence count. Expanding that row exposes the payloads for every occurrence, so aggregation reduces visual repetition without discarding evidence. Non-consecutive retries and failures whose error output differs remain separate.

### Control the outer activity disclosure as message-local UI state

The activity header is a keyboard-accessible toggle whose count badges remain visible in both states. A newly streaming turn opens the region while active work is present. A successful terminal turn collapses it when the user has not manually chosen a state. Manual choices are retained for that message so subsequent snapshots do not cause layout oscillation. Approval-required work overrides a collapsed preference and forces the content visible until the approval is resolved. A terminal failed assistant message opens initially but remains manually collapsible because the message-level error and failure count stay visible.

Using another outer `<details>` was rejected because nested native disclosures make programmatic approval overrides and user-preference tracking harder to express consistently. A semantic button with `aria-expanded` and `aria-controls` provides an explicit state boundary.

### Derive bounded safe previews from known structured fields

The renderer will inspect a small allowlist of string fields such as `command`, `cmd`, `path`, `query`, `pattern`, `action`, and `resource`, falling back to the tool label. Preview text is whitespace-normalized and length-bounded. Full input/output remains escaped text produced by JSON serialization inside a height-bounded `<pre>`.

Displaying entire inputs in the summary was rejected because commands can be long or sensitive and would recreate the density problem.

### Localize presentation without changing protocol enums

Tool names receive friendly frontend labels for known generic tools and otherwise retain their literal provider name. Statuses, count summaries, disclosure labels, and approval-related presentation use translation resources in all supported locales. Protocol status strings remain unchanged.

## Risks / Trade-offs

- **[Some providers may reuse ids incorrectly]** → Reconcile strictly within one assistant message; separate messages cannot overwrite each other.
- **[A status-only update could erase useful data]** → Merge defined fields and preserve earlier input/output when omitted.
- **[Recoverable failures may be visually overlooked]** → Keep a persistent failed count and a labeled failure disclosure; automatically open it when the assistant turn itself fails.
- **[Presentation grouping may hide a meaningful retry difference]** → Group only consecutive failures whose tool, bounded preview, and serialized output signatures match, and retain every occurrence in expanded details.
- **[Long activity arrays still cost render time when expanded]** → Keep details collapsed and bound each payload; virtualization is deferred until measured activity counts justify it.
- **[Automatic completion collapse may fight the user]** → Auto-collapse only before the user interacts; retain the message-local preference afterward and always override it for approvals.
- **[Existing persisted messages may already contain duplicate ids]** → Normalize by id at render time as a defensive compatibility layer in addition to fixing new event reduction.

## Migration Plan

Deploy the frontend reconciliation and renderer together. Existing protocol and stored message shapes remain compatible. Rollback restores the prior flat renderer without any data migration.
