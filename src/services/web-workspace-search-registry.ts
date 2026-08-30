/**
 * Which simulated inspections are running, and how many may.
 *
 * One registry for both search kinds, because the native side has one. Two independent counters here
 * would let a Quick Open and a content search look concurrent in the browser and supersede or refuse
 * each other on the desktop — a difference a panel gets written against without ever noticing, until
 * it is running on the other adapter.
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
 * megabyte of invented file content in the repository would make every unrelated fixture read slower
 * to no purpose.
 */
export function configureWebWorkspaceSearch(next: Partial<WebWorkspaceSearchLimits>): void {
  limits = { ...limits, ...next };
}

export function webWorkspaceSearchLimits(): WebWorkspaceSearchLimits {
  return limits;
}

export function resetWebWorkspaceSearch(): void {
  limits = { ...DEFAULT_LIMITS };
  nextGeneration = 1;
  inFlight = 0;
  currentGeneration.clear();
  cancelledGenerations.clear();
}

/**
 * Claims the search id for a new generation, superseding whatever held it.
 *
 * Claimed before admission is checked, so a cancel arriving while the request is refused still has a
 * slot to land on.
 */
export function claimWebSearchGeneration(searchId: string): number {
  const generation = nextGeneration++;
  currentGeneration.set(searchId, generation);
  return generation;
}

export function releaseWebSearchGeneration(searchId: string, generation: number): void {
  if (currentGeneration.get(searchId) === generation) currentGeneration.delete(searchId);
  cancelledGenerations.delete(generation);
}

export function webSearchGenerationIsCurrent(searchId: string, generation: number): boolean {
  return currentGeneration.get(searchId) === generation;
}

export function webSearchGenerationIsCancelled(generation: number): boolean {
  return cancelledGenerations.has(generation);
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

/** Whether capacity is exhausted. Checked before any simulated work begins. */
export function webSearchAdmissionExhausted(): boolean {
  return inFlight >= limits.maxConcurrent;
}

export function enterWebSearch(): void {
  inFlight += 1;
}

export function leaveWebSearch(): void {
  inFlight -= 1;
}

/**
 * Directories the simulated recursive search does not descend into.
 *
 * Deliberately a short list and deliberately the mock's own. The native policy holds eighteen names
 * and lives in one place on that side; restating all of them here would be a second list that drifts,
 * for a fixture set that contains three directories. What this exists to make observable is the
 * *behaviour* — that a recursive search skips somewhere on purpose and still reports `complete`,
 * because an ignored tree is a discovery rule rather than an omission.
 */
const SIMULATED_EXCLUSIONS = ["node_modules", "target", "dist"];

/** Whether a recursive search skips this workspace-relative path. */
export function webSearchSkipsPath(path: string): boolean {
  const [first] = path.split("/");
  return SIMULATED_EXCLUSIONS.includes(first);
}
