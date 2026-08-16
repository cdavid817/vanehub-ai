## Context

The Session Changes tab already obtains confined, bounded structured Git status/diffs from `workspaces` through the shared Agent service and matching Web/Tauri adapters. It is read-only and has no review aggregate, durable comments, stale anchors, guarded mutations, feedback projection, or automated findings. Existing `sessions`, `operations`, `permissions`, Agent runtime, and SQLite boundaries already own the adjacent lifecycles. Architecture Fitness Functions are archived and must remain green.

## Goals / Non-Goals

**Goals:**

- Deliver the complete P0 review loop from Agent-produced changes to comments, validation, decision, feedback, and safe revert.
- Preserve workspace confinement and service/adaptor parity while keeping long Git and check work asynchronous and bounded.
- Recover review state after restart and detect workspace drift without retaining full diff bodies in review storage or logs.
- Reuse current contexts and execution systems with mechanically verifiable boundaries.

**Non-Goals:**

- Git staging, merging, remote PR creation, Mission Control, a new run state machine, or Unified Context Engine work.
- A new bounded context, provider-specific feedback payload, second permission engine, or second Agent orchestration runtime.
- Rendering binary content or retaining unlimited diffs/comments/findings.

## Decisions

### 1. Review coordination uses existing bounded contexts

`sessions` owns the recoverable `ReviewSession`, comments, findings, decisions, and feedback receipt because the review is attached to an originating session. `workspaces` owns Git discovery, canonical path checks, bounded diff snapshots, fingerprints, and revert application. `operations` owns asynchronous action lifecycle and redacted diagnostics; `permissions` owns destructive approval. Cross-context use occurs through published APIs assembled in bootstrap.

Alternative: add a `code_review` context. Rejected because the first release has one consumer and its invariants naturally split across existing owners; it would duplicate workspace and session identities.

### 2. Persist metadata and anchors, regenerate bounded diff content

SQLite stores review identity, canonical workspace/worktree witness, base/head revision, working-tree fingerprint, file metadata/hashes, hunk fingerprints, comments/findings, decisions, action receipts, and timestamps. It does not store whole diff bodies. Opening/recovering a review asks `workspaces` to regenerate bounded content and marks or relocates anchors by fingerprint.

Alternative: persist every diff. Rejected due to sensitive-code duplication, unbounded storage, and stale copies. Pure ephemeral state is also rejected because R7 explicitly requires recovery and the repository already has suitable session persistence.

### 3. Anchors combine structural identity with bounded relocation

An anchor records normalized file path, side, old/new line range, normalized hunk header, hunk-content fingerprint, and a short bounded context fingerprint. Exact fingerprint match is current; a unique context match within the same file may relocate; zero or multiple matches are stale. Comment bodies are bounded UTF-8 text and never enter diagnostics.

### 4. Snapshot witnesses gate every destructive operation

Review creation computes an installation-stable review snapshot fingerprint from sorted changed-file metadata and per-file content witnesses. Revert requests include review id, file path, hunk fingerprint, and expected current file/worktree witness. Native code resolves the owning session root, rejects traversal/symlink escape, recomputes witnesses immediately before mutation, and applies a reverse patch with zero fuzz. File revert is an explicit bounded operation. Mismatch, ambiguous patch, binary, oversized, or unsupported state fails closed without partial mutation.

Accept hunk only records a review decision; it never stages Git content. Copy diff has no native mutation. Web/mock returns a simulated mutation receipt and mutates only its in-memory fixture.

### 5. Review actions use one runtime-neutral service contract

Shared TypeScript models and `AgentService` methods cover create/recover review, load a file, add/resolve/select comments, accept/revert, send feedback, launch an automated action, and observe action status. React never imports `invoke`. Tauri maps these methods to declared commands; Web implements deterministic asynchronous fixtures with the same state transitions and explicit `simulated: true` receipts.

### 6. Feedback and automated findings reuse current runtimes

Feedback is assembled below the UI as a provider-neutral structured envelope containing review/session ids, decision, and selected anchored comments. The existing session/Agent send boundary formats the human-readable prompt and retains metadata; stale comments require explicit user acknowledgement and are labelled stale. Automated actions are allowlisted (`review-agent`, `tests`, `security`) and run through existing Agent/tool/operation services. Their bounded structured output is normalized into findings. The MVP does not introduce multi-Agent routing.

### 7. Bounded loading and stable rendering

Status is capped by file count and aggregate metadata bytes. File diffs load on selection with independent per-file and aggregate budgets; the response exposes `binary`, `oversized`, `truncated`, and continuation metadata. Parsing and fingerprinting are single-pass with bounded allocations. The UI keeps the file list and prior content visible while a new file/action loads and uses memoized row models instead of rebuilding the full diff on comment edits.

### 8. Review UI extends Changes without adding a ninth tab

`Changes` becomes the Review Center entry to preserve `session-workspace-tabs`. Desktop uses a collapsible 220px file rail and main diff/review region; narrow layout switches the rail with an accessible control and permits code-region scrolling without page-level unrecoverable overflow. Shared semantic tokens cover both styles. Inline comment editors attach to selectable diff lines/hunks; file navigation and action toolbar remain reachable.

### 9. Diagnostics are metadata-only

Events cover review creation, bounded diff load, comment lifecycle, stale detection, revert outcome, feedback receipt, and automated action outcome. Allowed fields are safe ids, relative-path fingerprint (not raw absolute path), counts, byte sizes, durations, outcome/error category, and operation id. Code, diff bodies, comment/finding bodies, prompts, secrets, and raw command output are excluded before unified logging.

## Risks / Trade-offs

- [Git changes between witness check and patch write] → hold the workspace mutation guard across final witness validation and patch application, then recompute the resulting fingerprint.
- [Context relocation attaches to the wrong code] → relocate only on a unique same-file context match; otherwise mark stale.
- [SQLite migration or review persistence fails] → transactionally apply additive schema; keep existing session/chat usable and surface typed recovery failure.
- [Large repositories consume excessive memory or freeze UI] → cap file count/bytes, load per file, parse linearly, run native work off the main thread, and test structural budgets.
- [Automated tools emit arbitrary prose] → preserve bounded page-visible output but require an adapter-normalized finding schema; invalid output becomes an action error, not a fabricated finding.
- [Web simulation is mistaken for native mutation] → every receipt exposes `simulated`, and localized UI labels simulated actions.

## Migration Plan

1. Add versioned review tables and indexes in one transactional SQLite migration; existing databases gain empty review state.
2. Deploy Rust application/ports/adapters and declared commands before enabling frontend calls.
3. Extend shared contracts and both frontend adapters, then replace the Changes panel with the Review Center UI.
4. Existing sessions create reviews lazily; no backfill is required. Missing/deleted workspaces yield recoverable unavailable state.
5. Rollback code may ignore additive tables. A later forward migration owns table removal; rollback never destructively drops review data.

## Open Questions

- PR creation and Git staging remain explicit future deltas.
- Mission Control may consume the published review service later; this change adds no second consumer-specific API.
