import type {
  WorkspaceContentMatch,
  WorkspaceContentSearchResult,
  WorkspaceInspectionBudget,
  WorkspaceSearchCoverage,
} from "../types/session-workspace-inspection";

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

/** Simulated ceilings. Generous enough that the ordinary fixture search is genuinely complete. */
export interface WebWorkspaceSearchLimits {
  maxFiles: number;
  maxBytes: number;
  maxResults: number;
  /** How many searches may run at once before one is refused. */
  maxConcurrent: number;
}

const DEFAULT_LIMITS: WebWorkspaceSearchLimits = {
  maxFiles: 64,
  maxBytes: 1024 * 1024,
  maxResults: 200,
  maxConcurrent: 2,
};

let limits: WebWorkspaceSearchLimits = { ...DEFAULT_LIMITS };
let nextGeneration = 1;
let inFlight = 0;
/** Which generation currently answers for a search id. */
const currentGeneration = new Map<string, number>();
/** Generations an explicit cancel has reached. */
const cancelledGenerations = new Set<number>();

/**
 * Narrows the simulated ceilings so a caller can reach one.
 *
 * Exists because the alternative is fixtures large enough to exhaust a realistic budget, and a
 * megabyte of invented file content in the repository would make every unrelated fixture read
 * slower to no purpose.
 */
export function configureWebWorkspaceSearch(next: Partial<WebWorkspaceSearchLimits>): void {
  limits = { ...limits, ...next };
}

export function resetWebWorkspaceSearch(): void {
  limits = { ...DEFAULT_LIMITS };
  nextGeneration = 1;
  inFlight = 0;
  currentGeneration.clear();
  cancelledGenerations.clear();
}

/**
 * Asks a running search to stop, and says whether one was there to ask.
 *
 * `false` means "nothing to stop", not "you did something wrong". A caller cannot know whether their
 * cancel beat the search's own completion, and the desktop build answers the same way.
 */
export function cancelWebWorkspaceSearch(searchId: string): boolean {
  const generation = currentGeneration.get(searchId);
  if (generation === undefined) return false;
  cancelledGenerations.add(generation);
  return true;
}

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
  const generation = nextGeneration++;
  // Registered before admission is checked, so a cancel arriving while the request is refused still
  // has a slot to land on. Installing it also supersedes whatever held the id.
  currentGeneration.set(input.searchId, generation);

  if (inFlight >= limits.maxConcurrent) {
    // Busy is an answer, not an error. Nothing was scanned, so nothing is claimed about the
    // workspace — `unavailable` rather than `partial` says exactly that.
    currentGeneration.delete(input.searchId);
    return unavailable(generation, "inspection_busy");
  }

  inFlight += 1;
  try {
    const scan = await scanFixtures(input, files, generation);
    return deliver(generation, input.searchId, scan);
  } finally {
    inFlight -= 1;
    if (currentGeneration.get(input.searchId) === generation) {
      currentGeneration.delete(input.searchId);
    }
    cancelledGenerations.delete(generation);
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
  if (currentGeneration.get(searchId) !== generation) {
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

  const maxResults = Math.max(1, Math.min(input.limit ?? limits.maxResults, limits.maxResults));
  const matches: WorkspaceContentMatch[] = [];

  for (const [path, content] of Object.entries(files).sort(([left], [right]) =>
    left.localeCompare(right),
  )) {
    // Before the work rather than after, so a cancel issued while this was queued is observed before
    // another file is opened.
    await Promise.resolve();
    if (cancelledGenerations.has(generation)) return { matches, budget, reasonCode: "cancelled" };
    if (currentGeneration.get(input.searchId) !== generation) {
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
