import { CONTEXTUAL_COMMANDS } from "./contextual-commands";
import { DESTINATION_COMMANDS } from "./destination-commands";
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
 * 6.6 (Goal/Work Item/Evaluation providers) is not represented: design.md Decision 4 itself says
 * to ship Session/Project/Run first and add these "随后" (afterward), once their route adapters
 * exist. They don't yet — `PlanDestination`/`QualityDestination` don't consume `goalId`/
 * `workItemId`/`experimentId` from the URL at all (confirmed by reading both directly this
 * session), so a provider routing to one of those ids would link to state nothing reads. Deferred,
 * not forgotten.
 */
export const SEARCH_PROVIDERS: WorkbenchSearchProvider[] = [
  sessionSearchProvider,
  projectSearchProvider,
  runSearchProvider,
];

export const COMMANDS: WorkbenchCommand[] = [
  ...DESTINATION_COMMANDS,
  ...CONTEXTUAL_COMMANDS,
];
