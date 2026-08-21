## Scoped reference inventory

The implementation audit separated session Plan-mode safety from the retired standalone Plan execution vertical slice.

| Area | Retired live references | Retained references |
| --- | --- | --- |
| Frontend | Plan Center, `PlanService`, Plan adapters, PlanRun polling, associated-run navigation, `/plan` and `/plans` commands | OnePiece `ModeSelect`, session `executionMode`, `exit_plan_mode` approval |
| Workspace shell | `plans` destination, activity entry, lazy mount, visited and inspection state | Sessions fallback for retired or unknown routes |
| Native commands | Plan draft, approval, run, attempt, control, recovery, and query commands | `resolve_plan_exit` and session chat-configuration commands |
| Native runtime | `task_orchestration`, Plan driver, scheduler, structured Plan generator, Plan worktree preparation | Session policy resolution, read-only Plan tool enforcement, ordinary generation stop/recovery |
| Work Board | `plan` and `plan_run` sources, summaries, projection queries, reconciliation | `session` and `scheduled_task` sources |
| Operations | PlanRun retry, verify, pause/resume, cancel delegation, action projection | Generic canonical Run state and non-Plan owner controls |
| Goals and evidence | Live Plan progress probe; Plan-named Loop verification envelope | Historical Plan links remain readable as unresolvable; old evidence input names remain deserializable aliases |
| Persistence | Runtime reads and writes against Plan tables | Database-owned legacy schema replay, retained historical rows and worktree paths, retirement migration |

## Mutable change audit

Every unarchived change other than this change was searched for `Plan Center`, `PlanService`, `PlanRun`, task orchestration, `plan-management`, `plan-execution-runtime`, and retired Plans routes. No dependent mutable artifact was found, so no unrelated active change required revision. Historical artifacts under `openspec/changes/archive/` were not edited.

Current README, user-guide, developer-guide, context-map, Work Board, and screenshot-manifest references were updated to the session-owned Plan mode. The source-pinned architecture analysis and dated UX audit/implementation reports retain their old observations as historical evidence and now carry explicit historical-snapshot notices so they cannot be mistaken for current design.
