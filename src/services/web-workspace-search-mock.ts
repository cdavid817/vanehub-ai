import type {
  WorkspaceContentMatch,
  WorkspaceContentSearchResult,
  WorkspaceInspectionBudget,
  WorkspaceSearchCoverage,
} from "../types/session-workspace-inspection";
import {
  claimWebSearchGeneration,
  enterWebSearch,
  leaveWebSearch,
  releaseWebSearchGeneration,
  webSearchAdmissionExhausted,
  webSearchGenerationIsCancelled,
  webSearchGenerationIsCurrent,
  webWorkspaceSearchLimits,
} from "./web-workspace-search-registry";

/**
 * The browser build's content search, with the same stopping conditions as the desktop one.
 *
 * The point is not to pretend the browser has a filesystem. It does not, and the capabilities this
 * build reports say `simulated` for exactly that reason. The point is that cancellation,
 * supersession, admission and budget exhaustion are *contract*, not implementation detail: a panel
 * that only ever sees `complete` from one adapter will be written as though the other cannot stop
 * early, and the first time it does the panel will say "no matches" about a search that gave up.
 *
 * So the counters here are simulated and say so — they count fixture bytes, not disk reads — but the
 * shape of the answer is the shape the native adapter produces, including which reason code goes
 * with which coverage state.
 */

const NO_WORK: WorkspaceInspectionBudget = {
  directoriesVisited: 0,
  entriesVisited: 0,
  filesOpened: 0,
  bytesRead: 0,
  metadataOperations: 0,
  candidatesRetained: 0,
  resultsEmitted: 0,
  maxDepthReached: 0,
  unreadableEntries: 0,
};

interface Scan {
  matches: WorkspaceContentMatch[];
  budget: WorkspaceInspectionBudget;
  /** Why it stopped, or absent because it did not. */
  reasonCode?: string;
}

export interface WebWorkspaceSearchInput {
  query: string;
  searchId: string;
  limit?: number;
}

/**
 * Runs one simulated search over the fixture files.
 *
 * Yields between files rather than scanning straight through. A synchronous scan would finish before
 * any cancel could be issued, which would make every cancellation test pass by never testing
 * anything — the fixture would be reporting that the mechanism works because it was too fast to use.
 */
export async function runWebWorkspaceSearch(
  input: WebWorkspaceSearchInput,
  files: Record<string, string>,
): Promise<WorkspaceContentSearchResult> {
  // Claimed before admission is checked, so a cancel arriving while the request is refused still
  // has a slot to land on. Claiming it also supersedes whatever held the id.
  const generation = claimWebSearchGeneration(input.searchId);

  if (webSearchAdmissionExhausted()) {
    // Busy is an answer, not an error. Nothing was scanned, so nothing is claimed about the
    // workspace — `unavailable` rather than `partial` says exactly that.
    releaseWebSearchGeneration(input.searchId, generation);
    return unavailable(generation, "inspection_busy");
  }

  enterWebSearch();
  try {
    const scan = await scanFixtures(input, files, generation);
    return deliver(generation, input.searchId, scan);
  } finally {
    leaveWebSearch();
    releaseWebSearchGeneration(input.searchId, generation);
  }
}

/**
 * What a finished scan is allowed to hand back.
 *
 * Kept separate from the scan, and superseded results lose their matches, because that is what the
 * native side does: a replaced generation's matches are matches for a query the reader has already
 * retyped. The two adapters agreeing here is the only reason a panel can be written once.
 */
function deliver(generation: number, searchId: string, scan: Scan): WorkspaceContentSearchResult {
  if (!webSearchGenerationIsCurrent(searchId, generation)) {
    return {
      generation,
      coverage: { state: "partial", reasonCode: "superseded", budget: scan.budget },
      matches: [],
    };
  }
  return {
    generation,
    coverage: coverageFor(scan),
    matches: scan.matches,
  };
}

function coverageFor(scan: Scan): WorkspaceSearchCoverage {
  if (!scan.reasonCode) return { state: "complete", budget: scan.budget };
  return { state: "partial", reasonCode: scan.reasonCode, budget: scan.budget };
}

function unavailable(generation: number, reasonCode: string): WorkspaceContentSearchResult {
  return {
    generation,
    coverage: { state: "unavailable", reasonCode, budget: { ...NO_WORK } },
    matches: [],
  };
}

async function scanFixtures(
  input: WebWorkspaceSearchInput,
  files: Record<string, string>,
  generation: number,
): Promise<Scan> {
  const needle = input.query.trim().toLowerCase();
  const budget: WorkspaceInspectionBudget = { ...NO_WORK, directoriesVisited: 1, maxDepthReached: 1 };
  // An empty query would match every line of every file. Nothing is charged for it because nothing
  // is read: reporting a spend here would be inventing work to explain an answer that cost none.
  if (!needle) return { matches: [], budget };

  const limits = webWorkspaceSearchLimits();
  const maxResults = Math.max(1, Math.min(input.limit ?? limits.maxResults, limits.maxResults));
  const matches: WorkspaceContentMatch[] = [];

  for (const [path, content] of Object.entries(files).sort(([left], [right]) =>
    left.localeCompare(right),
  )) {
    // Before the work rather than after, so a cancel issued while this was queued is observed before
    // another file is opened.
    await Promise.resolve();
    if (webSearchGenerationIsCancelled(generation)) {
      return { matches, budget, reasonCode: "cancelled" };
    }
    if (!webSearchGenerationIsCurrent(input.searchId, generation)) {
      return { matches, budget, reasonCode: "superseded" };
    }

    budget.entriesVisited += 1;
    budget.metadataOperations += 1;
    if (budget.filesOpened >= limits.maxFiles) {
      return { matches, budget, reasonCode: "file_budget_exhausted" };
    }
    if (budget.bytesRead + content.length > limits.maxBytes) {
      return { matches, budget, reasonCode: "byte_budget_exhausted" };
    }

    budget.filesOpened += 1;
    budget.bytesRead += content.length;
    for (const [index, line] of content.split("\n").entries()) {
      const column = line.toLowerCase().indexOf(needle);
      if (column < 0) continue;
      if (matches.length >= maxResults) {
        return { matches, budget, reasonCode: "result_budget_exhausted" };
      }
      matches.push({
        path,
        line: index + 1,
        column: column + 1,
        snippet: line,
        snippetTruncated: false,
      });
      budget.resultsEmitted += 1;
    }
  }

  return { matches, budget };
}

/**
 * One page of a simulated path search.
 *
 * Shares the generation counter and the admission ceiling with content search, because the native
 * side shares one registry and one admission policy between them. A mock with two independent
 * counters would let a Quick Open and a content search look concurrent here and refuse each other
 * on the desktop, which is the kind of difference a panel is written against without noticing.
 *
 * The cursor is an offset into the fixture, bound to the query. A cursor applied to a different
 * query names a rank that ordering never produced, and that is the one refusal Quick Open can
 * actually hit — so it is the one this simulates.
 */
