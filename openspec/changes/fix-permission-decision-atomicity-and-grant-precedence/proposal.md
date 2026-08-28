# Make permission approval resolution atomic and grant selection deterministic

## Why

The permission system currently has two integrity gaps at the point where a human decision becomes executable authority.

First, `SqliteGrantRepository::find_matching` loads every row for one `(principal, action, resource)` without an ordering clause and returns the first row whose scope matches. Remembering another decision inserts another row instead of replacing the same logical grant. A session-, project-, and global-scoped row can therefore all match, and legacy or repeated rows within one scope can disagree. The effective decision depends on SQLite row order rather than an explicit security rule.

Second, the pending-approval command currently asks the waiting Agent or Claude hook to resume before `ApprovalBroker::finalize` removes the pending request, writes a remembered grant, and appends the audit row. Those writes use separate repositories and no shared transaction. A database failure can therefore leave an action already released while its grant or audit is absent, and removing the in-memory request before persistence also makes a safe retry impossible.

The same area contains two smaller races: first-use principal creation performs `SELECT` followed by `INSERT`, and concurrent calls can collide on the unique `agent_id`; internal evaluation failure falls back to `Ask` but does not always retain equivalent redacted evidence explaining that the result came from an infrastructure failure.

## What Changes

- Define a canonical remembered-grant key and deterministic matching order: exact Session, then exact Project, then Global. A more specific applicable scope wins; rows at the same canonical scope key are one revisioned value, never an unordered append-only set.
- Rebuild the grant table through a versioned migration, normalize or quarantine invalid legacy rows, deterministically deduplicate existing keys, add revision/update metadata, and enforce scope-specific uniqueness with SQLite indexes.
- Replace separate grant/audit writes in approval finalization with one `permissions`-owned atomic resolution repository operation that records the immutable resolution, its audit row, and an optional inactive remembered-grant intent.
- Claim an in-memory pending request with single-winner semantics, reserve the originating waiter/generation without resuming it, commit the durable resolution, and only then deliver the effect.
- Keep remembered grants inactive until the waiting Agent or hook acknowledges delivery. Delivery failure or an application restart never activates a grant or revives an ended generation.
- Make resolution and delivery idempotent through an immutable `resolution_id` and unique `request_id`; a double-click or retry returns the existing result and cannot execute twice.
- Route human decisions, timeout denials, and stale-generation rejection through one use case. `Allow` is never delivered before durable commit. A storage outage may use an emergency fail-closed `Deny` delivery so a generation does not hang, but it must never execute the action and must emit a redacted unified diagnostic.
- Replace principal `SELECT`-then-`INSERT` with an atomic get-or-create repository operation.
- Record redacted evidence for evaluation failures and preserve Web/mock parity for precedence, resolving state, idempotency, and failure semantics.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `permissions-core`: Define canonical grant identity and precedence, atomic resolution persistence, inactive grant intent, concurrency-safe principal creation, and fail-closed failure evidence.
- `permissions-approval`: Define single-winner resolution, commit-before-effect delivery, delivery acknowledgement, retry/idempotency, and restart reconciliation.
- `claude-code-permission-hook`: Require a committed immutable resolution before an `Ask` HTTP waiter is released and reject stale or duplicate delivery.

## Impact

- Affects the `permissions` domain/application ports, `ApprovalBroker`, evaluation flow, SQLite schema/repositories, bootstrap composition, timeout sweep, Claude hook wait registry, and the published `agent_runtime` approval delivery boundary.
- Adds a new migration chosen from the next free version after checking `main` and every active change. Existing published migration files are not edited.
- Adds one durable approval-resolution/delivery ledger and extends grants/audit with resolution metadata. Pending approvals remain process-local and are not restored as live work after restart.
- Keeps policy template names, action/resource mapping, MCP Ask floor, Tauri command names, and normal `Allow`/`Deny`/`Ask` frontend vocabulary unchanged.
- The approval service gains a resolving/delivery status needed to disable duplicate UI actions. Tauri and Web/mock adapters must expose the same typed contract and all new visible states require complete i18n parity.
- No raw tool input, shell command body, secret, credential, or unrestricted path is added to SQLite or unified logs.
