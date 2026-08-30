# Requirement-to-task-to-test traceability: `redesign-unified-workbench-ui`

Task 1.3. Living document — update the Task and Evidence columns whenever a requirement's owning
task changes or a test lands; do not let this drift from `tasks.md`. Suggested-gate and scenario
counts are inherited from the delivered `ACCEPTANCE_TEST_MATRIX.md` (verified against a full,
independent read of all 11 `specs/*/spec.md` files — the 89-requirement / 312-scenario totals in
that document check out exactly). "Task(s)" names the `tasks.md` section(s) primarily responsible;
most sections also need `§20` (responsive/a11y/i18n) and `§21` (tests/perf/visual) work before a
requirement is truly done — those two are the perpetual cross-cutting closers, not repeated below
per row. "Evidence" starts empty for everything except Milestone 0's own requirement (row 88,
partially covered by the baseline itself) and is filled in with real test file paths + commit
hashes as each requirement is actually implemented, never with "Unit, Component, ..." labels alone.

## agent-evaluation (`specs/agent-evaluation/spec.md`)

| # | Change | Requirement | Scenarios | Task(s) | Suggested gates | Evidence |
|---:|---|---|---:|---|---|---|
| 1 | ADDED | Evaluation experiment workflow | 4 | §18.1–18.5 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 2 | ADDED | Evaluation results data table | 4 | §18.6–18.7, 18.16 | Unit, Component, Web E2E, Desktop E2E, Visual, Perf, A11y, Contract | Not started |
| 3 | ADDED | Evaluation baseline and regression presentation | 4 | §18.8–18.10 | Unit, Component, Web E2E, Desktop E2E, Visual, Contract | Not started |
| 4 | ADDED | Multi-experiment comparison | 4 | §18.11 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 5 | ADDED | Explained evaluation outcomes | 4 | §18.12 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 6 | ADDED | Evaluation artifact evidence links | 3 | §18.13 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 7 | ADDED | Evaluation visibility-aware updates | 3 | §18.14 | Unit, Component, Web E2E, Desktop E2E, Perf, Contract | Not started |
| 8 | ADDED | Evaluation component boundaries | 2 | §18.2, §1.6 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |

## agent-mission-control (`specs/agent-mission-control/spec.md`)

| # | Change | Requirement | Scenarios | Task(s) | Suggested gates | Evidence |
|---:|---|---|---:|---|---|---|
| 9 | ADDED | Runs destination hierarchy | 3 | §16.1–16.2, §4 | Unit, Component, Web E2E, Desktop E2E, Visual, Contract | Not started |
| 10 | ADDED | Mission Control saved views | 3 | §16.6 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 11 | ADDED | Mission Control evidence navigation | 3 | §16.13, §9 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 12 | ADDED | Mission Control action locality | 3 | §16.14–16.15 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 13 | MODIFIED | Bounded Mission Control overview | 4 | §16.3–16.4 | Unit, Component, Web E2E, Desktop E2E, Perf, Contract | Not started |
| 14 | MODIFIED | Lazy and truthful Run detail | 6 | §16.8–16.12 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 15 | MODIFIED | Coalesced events and deterministic reconciliation | 4 | §16.16 | Unit, Component, Web E2E, Desktop E2E, Perf, Contract | Not started |
| 16 | MODIFIED | Compact accessible responsive presentation | 4 | §16.17, §20 | Unit, Component, Web E2E, Desktop E2E, Visual, A11y, Contract | Not started |

## chat-experience (`specs/chat-experience/spec.md`)

| # | Change | Requirement | Scenarios | Task(s) | Suggested gates | Evidence |
|---:|---|---|---:|---|---|---|
| 17 | ADDED | Progressive-disclosure run configuration | 5 | §10.15–10.18 | Unit, Component, Web E2E, Desktop E2E | Not started |
| 18 | ADDED | Inspectable conversation evidence | 4 | §10.7, §9.1, §9.6 | Unit, Component, Web E2E, Desktop E2E, Visual, A11y | Not started |
| 19 | ADDED | Windowed dynamic-height conversation history | 5 | §10.10–10.14 | Unit, Component, Web E2E, Desktop E2E, Perf, Contract | Not started |
| 20 | ADDED | Conversation content hierarchy | 4 | §10.4–10.6, 10.9 | Unit, Component, Web E2E, Desktop E2E, Visual | Not started |
| 21 | ADDED | Unified multi-seat navigation semantics | 4 | §10.20–10.21 | Unit, Component, Web E2E, Desktop E2E, Visual | Not started |
| 22 | ADDED | Composer and conversation responsive safety | 4 | §10.19, §20 | Unit, Component, Web E2E, Desktop E2E, Visual, Perf, A11y | Not started |
| 23 | ADDED | Conversation rendering cost isolation | 3 | §10.13–10.14 | Unit, Component, Web E2E, Desktop E2E, Visual, A11y | Not started |
| 24 | MODIFIED | Responsive message submission feedback | 4 | §10.19 | Unit, Component, Web E2E, Desktop E2E, Visual, Contract | Not started |

## goal-management (`specs/goal-management/spec.md`)

| # | Change | Requirement | Scenarios | Task(s) | Suggested gates | Evidence |
|---:|---|---|---:|---|---|---|
| 25 | ADDED | Searchable goal execution-target picker | 3 | §15.4–15.6 | Unit, Component, Web E2E, A11y | Not started |
| 26 | ADDED | Nonblocking goal mutations | 3 | §15.7–15.8 | Unit, Component, Web E2E, A11y, Contract | Not started |
| 27 | ADDED | Goal master-detail presentation | 3 | §15.1–15.3 | Unit, Component, Web E2E, Visual | Not started |
| 28 | ADDED | State-aware goal lifecycle actions | 3 | §15.3, 15.9 | Unit, Component, Web E2E, Perf | Not started |
| 29 | ADDED | Goal relationship overview | 3 | §15.10–15.11 | Unit, Component, Web E2E, A11y | Not started |
| 30 | ADDED | Responsive Goal Center navigation | 2 | §15.12, §20 | Unit, Component, Web E2E, Visual, A11y | Not started |

## loop-management-ui (`specs/loop-management-ui/spec.md`)

| # | Change | Requirement | Scenarios | Task(s) | Suggested gates | Evidence |
|---:|---|---|---:|---|---|---|
| 31 | ADDED | Loop definitions and runs route separation | 3 | §17.1–17.2, §4 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 32 | ADDED | Loop iteration Inspector selection | 3 | §17.11, §9 | Unit, Component, Web E2E, Desktop E2E, Visual, A11y, Contract | Not started |
| 33 | ADDED | Loop local mutation feedback | 3 | §17.14–17.15 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 34 | MODIFIED | Dedicated Loop Center | 3 | §17.3 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 35 | MODIFIED | Loop Center operational layout | 3 | §17.3 | Unit, Component, Web E2E, Desktop E2E, Visual, Contract | Not started |
| 36 | MODIFIED | Run phase and iteration monitoring | 4 | §17.8–17.10 | Unit, Component, Web E2E, Desktop E2E, Perf, Contract | Not started |
| 37 | MODIFIED | Persistent Loop run action header | 4 | §17.8 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 38 | MODIFIED | Decision-oriented iteration history | 3 | §17.10, 17.12 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 39 | MODIFIED | Decision-ready human acceptance panel | 5 | §17.13 | Unit, Component, Web E2E, Desktop E2E, Visual, Contract | Not started |

## main-layout-ui (`specs/main-layout-ui/spec.md`)

| # | Change | Requirement | Scenarios | Task(s) | Suggested gates | Evidence |
|---:|---|---|---:|---|---|---|
| 40 | ADDED | Session runtime panel | 4 | §8.6, 8.9–8.10 | Unit, Component, Web E2E, Desktop E2E, Visual | Not started |
| 41 | ADDED | Session route and return-context compatibility | 3 | §4.7–4.9, §8 | Unit, Component, Web E2E, Desktop E2E, Visual | Not started |
| 42 | ADDED | Contextual session status hierarchy | 3 | §10.1–10.2 | Unit, Component, Web E2E, Desktop E2E, Visual | Not started |
| 43 | MODIFIED | Workspace activity bar | 4 | §4.12–4.13, §5.1 | Unit, Component, Web E2E, Desktop E2E, Visual | Not started |
| 44 | MODIFIED | Three-panel workspace proportions | 4 | §5.2–5.10 | Unit, Component, Web E2E, Desktop E2E, Visual, A11y | Not started |
| 45 | MODIFIED | Sidebar session organization | 6 | §7.1–7.17 | Unit, Component, Web E2E, Desktop E2E, Visual, Perf, Contract | Not started |
| 46 | MODIFIED | Optimized information panel tabs | 5 | §9.1–9.17 | Unit, Component, Web E2E, Desktop E2E, Visual, Perf, A11y | Not started |
| 47 | MODIFIED | Create-session dialog | 6 | §11.1–11.14 | Unit, Component, Web E2E, Desktop E2E, Visual, A11y | Not started |
| 48 | MODIFIED | Declarative session workspace tab capabilities | 6 | §8.1–8.20 | Unit, Component, Web E2E, Desktop E2E, Visual, A11y | Not started |

## project-worktree-management (`specs/project-worktree-management/spec.md`)

| # | Change | Requirement | Scenarios | Task(s) | Suggested gates | Evidence |
|---:|---|---|---:|---|---|---|
| 49 | ADDED | First-class Projects and Workspaces destination | 3 | §13.1–13.4 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 50 | ADDED | Persistent workspace trust presentation | 3 | §13.6, 13.10 | Unit, Component, Web E2E, Desktop E2E, Visual, Contract | Not started |
| 51 | ADDED | Workspace contextual quick actions | 3 | §13.8 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 52 | ADDED | Unavailable workspace recovery | 3 | §13.6 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 53 | ADDED | Workspace relationship summary | 2 | §13.7 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 54 | ADDED | Responsive workspace management | 2 | §13.12, §20 | Unit, Component, Web E2E, Desktop E2E, Visual, A11y, Contract | Not started |

## scheduled-task-management (`specs/scheduled-task-management/spec.md`)

| # | Change | Requirement | Scenarios | Task(s) | Suggested gates | Evidence |
|---:|---|---|---:|---|---|---|
| 55 | ADDED | Scheduled task update and duplication | 4 | §19.7–19.9 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 56 | ADDED | Scheduled task run-now action | 3 | §19.10 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 57 | ADDED | Scheduled task run history UI | 4 | §19.11 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 58 | ADDED | Scheduled recurrence timezone and occurrence preview | 4 | §19.12–19.13 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 59 | ADDED | Scheduled runtime capability disclosure | 3 | §19.15 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 60 | ADDED | Localized scheduled recurrence labels | 3 | §19.14, §20 | Unit, Component, Web E2E, Desktop E2E, Visual, A11y, Contract | Not started |
| 61 | ADDED | Scheduled task action hierarchy | 3 | §19.16–19.17 | Unit, Component, Web E2E, Desktop E2E, Contract | Not started |
| 62 | MODIFIED | Scheduled task dialog | 5 | §19.1–19.3 | Unit, Component, Web E2E, Desktop E2E, A11y, Contract | Not started |

## settings-center-ui (`specs/settings-center-ui/spec.md`)

| # | Change | Requirement | Scenarios | Task(s) | Suggested gates | Evidence |
|---:|---|---|---:|---|---|---|
| 63 | ADDED | Searchable settings metadata registry | 3 | §12.1–12.3 | Unit, Component, Web E2E | Not started |
| 64 | ADDED | Cross-page field-level settings search | 4 | §12.4–12.7 | Unit, Component, Web E2E | Not started |
| 65 | ADDED | Unified settings save semantics | 4 | §12.10–12.11 | Unit, Component, Web E2E | Not started |
| 66 | ADDED | Settings unsaved-change protection | 3 | §12.12–12.13 | Unit, Component, Web E2E | Not started |
| 67 | ADDED | Workflow-grouped settings navigation | 3 | §12.9 | Unit, Component, Web E2E, Desktop E2E, Visual | Not started |
| 68 | ADDED | Settings danger and sensitivity hierarchy | 3 | §12.14–12.15 | Unit, Component, Web E2E | Not started |
| 69 | ADDED | Copyable safe settings diagnostics | 2 | §12.19 | Unit, Component, Web E2E | Not started |
| 70 | MODIFIED | Lazy settings module loading | 6 | §12.17 | Unit, Component, Web E2E | Not started |
| 71 | MODIFIED | Polished settings visual system | 4 | §12.8, 12.18 | Unit, Component, Web E2E, Visual | Not started |

## unified-todo-board (`specs/unified-todo-board/spec.md`)

| # | Change | Requirement | Scenarios | Task(s) | Suggested gates | Evidence |
|---:|---|---|---:|---|---|---|
| 72 | ADDED | Unified board toolbar and saved views | 4 | §14.1, 14.3–14.5 | Unit, Component, Web E2E, Visual, A11y | Not started |
| 73 | ADDED | Nonblocking work-item mutations | 3 | §14.10–14.11 | Unit, Component, Web E2E, Contract | Not started |
| 74 | ADDED | Work-item editor sheet | 3 | §14.2 | Unit, Component, Web E2E, A11y | Not started |
| 75 | ADDED | Canonical and accessible stage movement | 3 | §14.8–14.9 | Unit, Component, Web E2E, A11y | Not started |
| 76 | ADDED | Board batch management | 3 | §14.12 | Unit, Component, Web E2E | Not started |
| 77 | ADDED | Responsive stage-list presentation | 2 | §14.13, §20 | Unit, Component, Web E2E, Visual | Not started |
| 78 | ADDED | Bounded work-item card metadata | 3 | §14.6–14.7 | Unit, Component, Web E2E, Perf | Not started |
| 79 | ADDED | Optional board WIP guidance | 2 | §14.14 | Unit, Component, Web E2E, Visual | Not started |

## workbench-design-system-ui (`specs/workbench-design-system-ui/spec.md`)

| # | Change | Requirement | Scenarios | Task(s) | Suggested gates | Evidence |
|---:|---|---|---:|---|---|---|
| 80 | ADDED | Task-domain workbench navigation | 3 | §4.1–4.3, 4.12, §5.1 | Unit, Component, Web E2E, Visual, A11y | Not started |
| 81 | ADDED | Shared destination layout primitives | 2 | §3.1–3.2 | Unit, Component, Web E2E, Visual, A11y | Not started |
| 82 | ADDED | Container-responsive pane composition | 3 | §5.3–5.10 | Unit, Component, Web E2E, Visual, A11y | Not started |
| 83 | ADDED | Global command center | 4 | §6.1–6.14 | Unit, Component, Web E2E, Visual, A11y | Not started |
| 84 | ADDED | Explicit page lifecycle policy | 3 | §5.11–5.17, §3.13 | Unit, Component, Web E2E, Visual, Perf, A11y | Not started |
| 85 | ADDED | Unified asynchronous view states | 4 | §3.13–3.14 | Unit, Component, Web E2E, Visual, Perf, A11y, Contract | Not started |
| 86 | ADDED | Shared keyboard interaction models | 4 | §3 (all), §20.7 | Unit, Component, Web E2E, Visual, A11y | Not started |
| 87 | ADDED | Semantic theme and localization parity | 3 | §2.1–2.14, §20.13–20.17 | Unit, Component, Web E2E, Visual, A11y | Not started |
| 88 | ADDED | Structural frontend performance budgets | 3 | §0.9 (fixtures), §21.7–21.16 | Unit, Component, Web E2E, Visual, Perf, A11y, Contract | Fixtures done: `src/testing/fixtures/` (task 0.9). Perf budget tests not started. |
| 89 | ADDED | Workbench visual regression contract | 3 | §21.17–21.21 | Unit, Component, Web E2E, Desktop E2E, Visual, A11y, Contract | Baseline screenshots done: `docs/ui-redesign/screenshots/baseline/` (task 0.7). Regression matrix itself not started. |

## Reconciliation notes

- The delivered `ACCEPTANCE_TEST_MATRIX.md` and `IMPLEMENTATION_GUIDE.md` (outside this repo, under
  the delivery package root) are strong starting drafts, independently verified against a full read
  of all 11 spec deltas — the 89/312 totals check out exactly. They are not duplicated into this
  repo verbatim; this file and `implementation-notes.md` carry forward only what task 1.2/1.3
  require as a tracked, versioned repo artifact.
- Row 88's fixtures and row 89's screenshots are Milestone 0 evidence, not full requirement
  implementations — recorded honestly as partial, not marked done.
