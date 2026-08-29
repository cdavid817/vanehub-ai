## Context

See `proposal.md` for motivation. The shared React dialog currently combines data orchestration, form controls, recurrence rendering, and task rows in one near-limit file. It calls the existing scheduled-task service correctly, but only create has an operation state, recurrence details include hard-coded English weekday labels, and the desktop automation suite has no scheduled-task lifecycle path. The native commands and SQLite model already support the requested CRUD behavior, so this change does not need a backend contract or schema migration.

## Goals / Non-Goals

**Goals:**

- Keep orchestration small and split form/list presentation into focused components below the production-file line limit.
- Give every asynchronous action a visible, scoped state while retaining successful data.
- Make recurrence input and summaries locale-aware, accessible, and valid before service submission.
- Exercise the real Tauri command and isolated SQLite boundaries from WebdriverIO through rendered UI.

**Non-Goals:**

- Adding manual "run now", editing, run-history browsing, or new recurrence types.
- Invoking real model providers or reading developer credentials during automation.
- Changing scheduled-task service methods, native command names, scheduler semantics, or persistence schema.

## Decisions

### Split orchestration from task-list and form presentation

`ScheduledTasksDialog` will own service calls and top-level state. A task-list component will render summaries and per-row actions, while a form component will own recurrence control presentation through controlled props. This keeps runtime access in one component, makes dense responsive layout easier to reason about, and avoids expanding the existing file beyond 300 lines. Keeping everything in the current file was rejected because UI refinement would turn the current near-limit file into a new lint exception.

### Validate a normalized draft before submission

The form will derive field-level validity from trimmed text, stable Agent selection, and recurrence ranges already supported by the domain. The submit action remains disabled until valid, and accessible error/help text is associated with the relevant controls. The service/native layer remains authoritative and its errors are still shown. Relying only on native rejection was rejected because it allows invalid empty or transient numeric values to cross the service boundary and provides poor field context.

### Track refresh separately from scoped mutations

Initial loading, background refresh, creation, and a single per-task mutation will be represented separately. Existing tasks remain rendered during refresh, affected controls are disabled while their mutation is pending, and unsuccessful mutations do not discard list or form context. Optimistic native state changes were rejected because the current service returns authoritative records cheaply and rollback would add complexity without a perceived latency benefit.

### Use semantic styling and localized recurrence metadata

The redesign will use existing semantic tokens and shared controls, a compact two-column desktop layout that collapses safely at narrow widths, status icons/text, and scroll containment inside the dialog. Weekdays, units, field labels, and recurrence summaries will be derived through i18n rather than stable English constants. No page-local palette or inline style will be introduced.

### Add a dedicated scheduled-task desktop layer

The desktop orchestrator will expose a scheduled-task WebdriverIO layer with its own config, spec directory, result directory, and deterministic CLI fixture environment. The test will use accessible labels to open the dialog, submit an `opencode` task, assert native records through Tauri IPC, toggle state, delete it, verify focus/Escape, and check for fatal UI errors. The fixture PATH guarantees that Agent detection is deterministic and prevents model calls; it does not replace scheduled-task commands or SQLite.

Shared navigation helpers will explicitly activate the required activity before searching for surface-specific controls. Increasing timeouts was rejected because the captured failure is a wrong-surface assumption, not slow rendering.

## Risks / Trade-offs

- [Additional desktop layer increases verification time] → Reuse the already-built artifact and keep one application instance for the CRUD lifecycle.
- [Text-based localized selectors can drift with copy changes] → Pin zh-CN as required, prefer accessible labels and form labels, and keep selectors close to user-observable contracts.
- [CLI detection can delay UI readiness] → Use deterministic fixture executables for configured CLI identities and wait on the dialog/service result rather than global detection completion.
- [A narrow viewport can change visual ordering] → Keep DOM order logical, use CSS grid only for presentation, and add component assertions plus native screenshot inspection.

## Migration Plan

No data migration is required. Ship the shared React/i18n changes and the desktop verification layer together. Rollback consists of reverting these files; existing scheduled-task rows remain compatible because service and persistence contracts do not change.
