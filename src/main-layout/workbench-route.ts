/**
 * Replaces `workspace-route.ts`'s six flat destinations (design.md Decision 1) with the five
 * stable business domains; Loops/Schedules/Board/Goals/Evaluations/Mission Control move to
 * secondary sections inside Runs/Plan/Quality instead of each owning a primary route.
 */
export type WorkbenchDestination = "sessions" | "projects" | "runs" | "plan" | "quality";

export const workbenchDestinations: WorkbenchDestination[] = ["sessions", "projects", "runs", "plan", "quality"];

export type RunsSection =
  | { section: "attention" | "active" | "history"; runId?: string }
  | { section: "loops"; definitionId?: string; loopRunId?: string }
  | { section: "schedules"; scheduleId?: string };

export type PlanSection =
  | { section: "board"; viewId?: string; workItemId?: string }
  | { section: "goals"; goalId?: string };

export type QualitySection =
  | { section: "evaluations"; experimentId?: string; comparisonIds?: string[] };

const createSessionSegment = "new";

export type WorkbenchLocation =
  | { destination: "sessions"; sessionId: string | null; creatingSession: boolean }
  | { destination: "projects"; projectId?: string }
  | ({ destination: "runs" } & RunsSection)
  | ({ destination: "plan" } & PlanSection)
  | ({ destination: "quality" } & QualitySection);

export const defaultWorkbenchLocation: WorkbenchLocation = {
  destination: "sessions",
  sessionId: null,
  creatingSession: false,
};

/**
 * A route naming an object outside its owner's actual scope (Decision 5: "无权限或不属于 route
 * scope 时,显示明确状态并回退列表"). Distinct from a plain missing/unknown route, which falls back
 * to the default location the way an unrecognized destination always has — this is for a
 * syntactically valid route whose target turns out to be gone, restricted, or foreign once
 * checked against real data, which the parser alone cannot know.
 */
export type RouteValidationState = "not-found" | "deleted" | "restricted" | "stale";

function isDestination(value: string): value is WorkbenchDestination {
  return (workbenchDestinations as string[]).includes(value);
}

function segment(value: string | undefined): string | undefined {
  return value ? decodeURIComponent(value) : undefined;
}

function parseRunsSection(segments: string[]): RunsSection {
  const [section, first] = segments;
  if (section === "loops") return { section: "loops", definitionId: segment(first), loopRunId: segment(segments[2]) };
  if (section === "schedules") return { section: "schedules", scheduleId: segment(first) };
  if (section === "active" || section === "history") return { section, runId: segment(first) };
  return { section: "attention", runId: segment(first) };
}

function parsePlanSection(segments: string[], search: URLSearchParams): PlanSection {
  if (segments[0] === "goals") return { section: "goals", goalId: segment(segments[1]) };
  const viewId = search.get("view") ?? undefined;
  return { section: "board", viewId, workItemId: segment(segments[1]) };
}

function parseQualitySection(segments: string[], search: URLSearchParams): QualitySection {
  const comparisonIds = search.get("compare")?.split(",").filter(Boolean);
  return { section: "evaluations", experimentId: segment(segments[1]), comparisonIds: comparisonIds?.length ? comparisonIds : undefined };
}

/**
 * Unknown destinations and malformed shapes fall back to sessions rather than an empty region,
 * matching the existing `parseWorkspaceLocation` precedent — the workspace has no bare "not
 * found" surface, and a blank panel reads as a broken app. A syntactically valid route whose
 * target does not actually exist is a separate case (`RouteValidationState`), resolved once real
 * data is available, not during parsing.
 */
export function parseWorkbenchLocation(pathname: string, search: URLSearchParams = new URLSearchParams()): WorkbenchLocation {
  const segments = pathname.split("/").filter(Boolean);
  if (segments[0] !== "workspace") return defaultWorkbenchLocation;
  const destination = segments[1];
  if (!destination || !isDestination(destination)) return defaultWorkbenchLocation;

  if (destination === "sessions") {
    const detail = segment(segments[2]);
    return { destination, sessionId: detail === createSessionSegment ? null : (detail ?? null), creatingSession: detail === createSessionSegment };
  }
  if (destination === "projects") return { destination, projectId: segment(segments[2]) };
  if (destination === "runs") return { destination, ...parseRunsSection(segments.slice(2)) };
  if (destination === "plan") return { destination, ...parsePlanSection(segments.slice(2), search) };
  return { destination, ...parseQualitySection(segments.slice(2), search) };
}

function runsPath(location: Extract<WorkbenchLocation, { destination: "runs" }>): string {
  if (location.section === "loops") {
    const parts = ["loops", location.definitionId, location.definitionId ? location.loopRunId : undefined]
      .filter((value): value is string => Boolean(value));
    return `/workspace/runs/${parts.map(encodeURIComponent).join("/")}`;
  }
  if (location.section === "schedules") {
    return `/workspace/runs/schedules${location.scheduleId ? `/${encodeURIComponent(location.scheduleId)}` : ""}`;
  }
  return `/workspace/runs/${location.section}${location.runId ? `/${encodeURIComponent(location.runId)}` : ""}`;
}

function planPath(location: Extract<WorkbenchLocation, { destination: "plan" }>): string {
  if (location.section === "goals") {
    return `/workspace/plan/goals${location.goalId ? `/${encodeURIComponent(location.goalId)}` : ""}`;
  }
  const base = `/workspace/plan/board${location.workItemId ? `/${encodeURIComponent(location.workItemId)}` : ""}`;
  return location.viewId ? `${base}?view=${encodeURIComponent(location.viewId)}` : base;
}

function qualityPath(location: Extract<WorkbenchLocation, { destination: "quality" }>): string {
  const base = `/workspace/quality/evaluations${location.experimentId ? `/${encodeURIComponent(location.experimentId)}` : ""}`;
  return location.comparisonIds?.length ? `${base}?compare=${location.comparisonIds.map(encodeURIComponent).join(",")}` : base;
}

export function workbenchPath(location: WorkbenchLocation): string {
  if (location.destination === "sessions") {
    if (location.creatingSession) return `/workspace/sessions/${createSessionSegment}`;
    return location.sessionId ? `/workspace/sessions/${encodeURIComponent(location.sessionId)}` : "/workspace/sessions";
  }
  if (location.destination === "projects") {
    return location.projectId ? `/workspace/projects/${encodeURIComponent(location.projectId)}` : "/workspace/projects";
  }
  if (location.destination === "runs") return runsPath(location);
  if (location.destination === "plan") return planPath(location);
  return qualityPath(location);
}

const RETURN_TO_PARAM = "returnTo";

/**
 * Decision 5: "禁止用任意外部 URL 作为 return target." Safety comes from the return type, not from
 * pattern-matching the input string — a token only ever becomes a `WorkbenchLocation` by round-
 * tripping through `parseWorkbenchLocation`, which cannot produce anything but one of the five
 * known destinations. A malicious or malformed token fails closed to `null` (no return context),
 * never to an attacker-controlled path.
 */
export function decodeReturnToken(token: string | null): WorkbenchLocation | null {
  if (!token) return null;
  let decoded: string;
  try {
    decoded = decodeURIComponent(token);
  } catch {
    return null;
  }
  if (!decoded.startsWith("/workspace/")) return null;
  const [pathname, search] = decoded.split("?");
  return parseWorkbenchLocation(pathname, new URLSearchParams(search));
}

export function extractReturnTo(search: URLSearchParams): WorkbenchLocation | null {
  return decodeReturnToken(search.get(RETURN_TO_PARAM));
}

/** Appends a validated internal return token to `path`, safe to hand to any evidence surface. */
export function withReturnTo(path: string, from: WorkbenchLocation): string {
  const separator = path.includes("?") ? "&" : "?";
  return `${path}${separator}${RETURN_TO_PARAM}=${encodeURIComponent(workbenchPath(from))}`;
}

/**
 * A new key rather than reusing `vanehub.workspace.location.v1`: the old value's shape (a flat
 * six-destination path) cannot round-trip through `parseWorkbenchLocation` meaningfully, and this
 * is a last-visited-page convenience cache, not user-authored data — worth a clean slate rather
 * than a migration.
 */
const storageKey = "vanehub.workbench.location.v1";

/** A half-finished creation is not a place worth returning to. */
export function rememberWorkbenchLocation(location: WorkbenchLocation): void {
  if (typeof localStorage === "undefined") return;
  const settled = location.destination === "sessions" ? { ...location, creatingSession: false } : location;
  localStorage.setItem(storageKey, workbenchPath(settled));
}

export function recallWorkbenchPath(): string {
  if (typeof localStorage === "undefined") return workbenchPath(defaultWorkbenchLocation);
  const stored = localStorage.getItem(storageKey);
  if (!stored?.startsWith("/workspace/")) return workbenchPath(defaultWorkbenchLocation);
  const [pathname, search] = stored.split("?");
  return workbenchPath(parseWorkbenchLocation(pathname, new URLSearchParams(search)));
}
