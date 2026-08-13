## Why

OnePiece already persists structured Plan drafts, runs serial SubTask attempts, and records command evidence, but planning has no project discovery context, execution advances only when the user clicks “execute next”, and failed verification does not feed an automated repair attempt. Completing this path now turns the existing foundation into a clear Claude Code-style Plan-to-Agent workflow that can safely continue to an evidence-backed review boundary.

## What Changes

- Add a OnePiece Plan phase that gathers a bounded, read-only, redacted project context before generating a structured task graph, while preserving the rule that planning cannot mutate the workspace.
- Present Plan and Agent as explicit session modes with persistent icon-and-text status, different composer actions, and an approval transition that freezes the reviewed Plan before any write-capable execution begins.
- Replace user-driven “execute next” progression with a native durable Plan driver that continues serial scheduling in the background until paused, cancelled, blocked, failed, or ready for final acceptance.
- Make SubTask acceptance criteria evidence-bearing, require meaningful verification instead of treating an empty validation set as success, and expose structured validation configuration during Plan review.
- Add bounded repair attempts for eligible execution or verification failures, carrying forward sanitized failure evidence while retaining every prior Attempt and enforcing an approved retry limit.
- Add Plan-level final verification after all SubTasks pass, then retain explicit user acceptance before completion; no automatic commit, merge, push, target-branch mutation, or worktree removal is introduced.
- Project `verifying` and `repairing` as first-class PlanRun states, and persist an optional originating OnePiece session association so mode transitions and navigation are based on native state rather than UI inference.
- Preserve deterministic Web/mock behavior and matching typed service contracts without claiming native provider, Git, command, or SQLite execution.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-chat-configuration`: Define visible OnePiece Plan/Agent mode semantics, read-only planning boundaries, approval-driven transition, and mode behavior while a PlanRun is active.
- `plan-management`: Add bounded project discovery context, evidence-linked acceptance criteria, editable verification configuration, and an approval snapshot that captures retry and final-verification policy.
- `plan-execution-runtime`: Add the durable background driver, bounded repair loop, non-vacuous SubTask verification, Plan-level final verification, and action-required terminal behavior.
- `onepiece-native-agent`: Add project-aware planning and repair requests that remain workspace-bounded, profile-scoped, and free of copied credentials or unbounded failure output.
- `frontend-runtime-architecture`: Extend Plan service contracts and Tauri/Web adapter parity for mode presentation, background progress, validation editing, repair evidence, and final verification state.
- `agent-execution-observability`: Correlate driver cycles, repair attempts, and final verification while retaining metadata-only logging and bounded user-facing evidence.

## Impact

- **Desktop runtime:** Extends the Rust `task_orchestration` context, OnePiece planning/execution ports, SQLite schema and repositories, guarded verification, startup recovery, background lifecycle assembly, and session-to-PlanRun association records.
- **Frontend:** Updates the OnePiece composer mode selector and status treatment, Plan review editor, run progress and evidence views, and removes the UI as the authority that advances each SubTask.
- **Runtime adapters:** Extends shared TypeScript Plan/session contracts, the Tauri Plan adapter, and deterministic Web/mock behavior; React remains isolated from Tauri commands and SQLite.
- **Safety and logging:** Keeps planning read-only, runs validation through existing guarded operations, writes diagnostics through unified logging, and excludes goals, prompts, credentials, raw tool payloads, and unbounded command output from persistent diagnostics.
- **Compatibility:** Existing completed Plan records and non-OnePiece sessions remain readable. Additive migrations provide defaults for new execution policy fields; no new state library, UI framework, database, or package manager is introduced.
