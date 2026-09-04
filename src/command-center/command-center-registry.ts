import { CONTEXTUAL_COMMANDS } from "./contextual-commands";
import { DESTINATION_COMMANDS } from "./destination-commands";
import { goalSearchProvider } from "./goal-search-provider";
import { projectSearchProvider } from "./project-search-provider";
import { runSearchProvider } from "./run-search-provider";
import { sessionSearchProvider } from "./session-search-provider";
import type { WorkbenchCommand, WorkbenchSearchProvider } from "./command-center-types";

/**
 * 6.1. A plain aggregation, not a mutable registration API (`registerProvider(...)`) — the set of
 * providers/commands is fixed at build time, the same way `settings-pages.ts` aggregates every
 * settings page into one array rather than having each page call a `registerPage()` side effect on
 * import. "No direct cross-domain mutation dependency" (6.1's own wording) holds because each
 * provider/command module only exports its own value; nothing here writes into another domain's
 * module, and nothing but this one file needs to know the full set exists.
 *
 * Work Item and Evaluation providers (the rest of the original 6.6 scope) stay unrepresented, each
 * genuinely still blocked for its own reason (re-verified via `add-goal-command-center-provider`):
 * `WorkBoard` has no injectable initial-selection prop for `workItemId`, and `EvaluationCenter`'s
 * "selected" concept (a run attempt) does not map cleanly onto "experiment" without a real design
 * decision this registry should not make on its own. Goal is no longer in that boat — `goalId` is
 * consumed end to end (`PlanDestination` wires it into `GoalCenter`, task 15.1) — so its provider
 * is registered below.
 */
export const SEARCH_PROVIDERS: WorkbenchSearchProvider[] = [
  sessionSearchProvider,
  projectSearchProvider,
  runSearchProvider,
  goalSearchProvider,
];

export const COMMANDS: WorkbenchCommand[] = [
  ...DESTINATION_COMMANDS,
  ...CONTEXTUAL_COMMANDS,
];
