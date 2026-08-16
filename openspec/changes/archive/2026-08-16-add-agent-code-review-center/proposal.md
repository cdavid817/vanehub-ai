## Why

VaneHub can already inspect bounded Git status and structured diffs, but an Agent's edits still lack a durable, safe review loop for anchored comments, guarded hunk decisions, feedback delivery, and automated findings. This change turns the existing Session Changes surface into the P0 Agent Code Review Center without duplicating workspace Git inspection, permissions, operations, or Agent runtime architecture.

## What Changes

- Add recoverable review sessions tied to the originating session and canonical workspace/worktree snapshot, with review files, fingerprinted hunk/line anchors, comments, findings, and decisions.
- Extend the existing Session Changes surface into a responsive Review Center with changed-file navigation, unified/split diffs, inline comments, stale-anchor presentation, copy, accept, guarded hunk/file revert, feedback selection, and loading/error/empty/oversize/binary states.
- Reuse the `workspaces` context for confined Git snapshots, bounded diff loading, fingerprint verification, and surgical revert; destructive mutations require explicit confirmation/approval and fail closed when the witness is stale.
- Send structured, provider-neutral review feedback through the existing session/Agent service boundary while preserving file, line, hunk, and stale metadata.
- Project Review Agent, test, and security-check results into normalized review findings by reusing existing Agent/tool/operation execution rather than adding another orchestration engine.
- Persist review recovery state in SQLite and emit metadata-only redacted review lifecycle diagnostics through unified operations/logging; code, comment bodies, and full diffs are excluded from diagnostic logs.
- Keep Web/mock and Tauri adapters contract-compatible: Web provides deterministic simulated review and never claims real Git mutation; Tauri performs bounded native Git operations asynchronously.
- Add deterministic security, contract, UI, visual, desktop, and bounded-loading/performance coverage. PR creation and later roadmap items remain out of scope.

## Capabilities

### New Capabilities

- `agent-code-review`: Defines review lifecycle, snapshot and anchor integrity, comments/findings/decisions, guarded revert, feedback delivery, automated review actions, persistence, recovery, observability, adapter parity, and performance/security bounds.

### Modified Capabilities

- `session-project-inspection`: Extends read-only structured Git inspection with review snapshots, stable fingerprints, per-file bounded loading, and explicitly guarded file/hunk mutation owned by `workspaces`.
- `session-workspace-tabs`: Upgrades the existing Changes tab into the Review Center entry and responsive review workflow while preserving the eight-tab lifecycle.
- `frontend-runtime-architecture`: Adds one runtime-neutral review contract implemented consistently by Tauri and Web/mock adapters.
- `native-runtime-architecture`: Assigns review persistence and feedback coordination to existing contexts while keeping Git/path policy in `workspaces`, review records in `sessions`, lifecycle telemetry in `operations`, and approval in `permissions`.
- `unified-log-management`: Adds redacted metadata-only review lifecycle events and explicitly excludes workspace code, diff bodies, and review comment bodies from diagnostics.

## Impact

- Frontend: extends `AgentService`, shared review models, Tauri/Web clients, Session Changes UI, i18n resources, and Vitest/Playwright visual coverage.
- Native: extends the existing `workspaces` application/ports/infrastructure and commands for bounded snapshots/revert, `sessions` persistence/application APIs for review records and feedback projection, and existing operations/permissions integration.
- Storage: adds additive SQLite review tables and indexes with compatibility/migration tests; no existing data is rewritten or removed.
- Runtime: both desktop and Web are affected. Desktop performs real bounded Git inspection/mutation; Web remains deterministic and simulated.
- Security: canonical workspace confinement, traversal/symlink rejection, UTF-8 and byte/count bounds, stale witness checks, destructive confirmation/approval, and redacted metadata-only logging are mandatory.
- Dependencies: reuses the archived Architecture Fitness Functions change and current service-contract/DDD checks. No new UI framework, state library, bounded context, orchestration engine, or provider-specific payload is introduced.
