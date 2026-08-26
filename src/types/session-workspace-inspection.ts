export type WorkspaceInspectionProviderKind = "local" | "ssh" | "simulated";

/**
 * A capability the provider either has or does not. `reasonCode` explains an absence in terms the
 * UI can localize; a blank panel with a generic error is the outcome this type exists to prevent.
 */
export interface CapabilityState {
  available: boolean;
  reasonCode?: string;
  remediation?: string;
}

/**
 * How change detection works for this provider. `none` is honest about a target that cannot report
 * changes at all, which is different from one that simply has not reported any yet.
 */
export type WorkspaceWatchMode = "native" | "polling" | "event-derived" | "none";

export interface WorkspaceInspectionCapabilities {
  provider: WorkspaceInspectionProviderKind;
  listFiles: CapabilityState;
  readTextFiles: CapabilityState;
  searchFiles: CapabilityState;
  gitStatus: CapabilityState;
  gitDiff: CapabilityState;
  watchMode: WorkspaceWatchMode;
}

/** One thing Quick Open found. `kind` is carried because a reader acts on the two differently. */
export interface WorkspacePathMatch {
  name: string;
  /** Workspace-relative, with forward slashes. */
  path: string;
  kind: "file" | "directory";
}

/**
 * How much of the workspace a search examined.
 *
 * Separate from the cursor, and the distinction is the point: a cursor says more matches follow,
 * coverage says part of the workspace was never looked at. Paging fixes the first and can never fix
 * the second, so a reader who reached the end of the list still needs to know which end it was.
 */
export interface WorkspaceSearchCoverage {
  state: "complete" | "partial" | "unavailable";
  reasonCode?: string;
}

export interface WorkspacePathSearchResult {
  coverage: WorkspaceSearchCoverage;
  matches: WorkspacePathMatch[];
  /** Absent on the last page, never an empty string. */
  nextCursor?: string;
}

/** One position inside one file. */
export interface WorkspaceContentMatch {
  /** Workspace-relative, with forward slashes. */
  path: string;
  /** 1-based, because that is what every editor and every error message uses. */
  line: number;
  /** 1-based and counted in characters, not bytes. */
  column: number;
  /** A bounded, control-free slice of the matching line. */
  snippet: string;
  /**
   * Whether the line was cut to fit. Separate from the search's own bound, because a complete
   * result made of trimmed lines is still complete.
   */
  snippetTruncated: boolean;
}

export interface WorkspaceContentSearchResult {
  coverage: WorkspaceSearchCoverage;
  matches: WorkspaceContentMatch[];
}

/**
 * Which mechanism noticed a workspace change.
 *
 * Carried so a reader can weigh the notice, not so it can decide what to refresh. A poll's answer
 * is true as of the poll; a write this application performed is exact. Same instruction, different
 * guarantees.
 */
export type WorkspaceInvalidationSource = "watch" | "poll" | "execution-evidence";

/** What happened to a path, when that is known. `unknown` is an answer, not a placeholder. */
export type WorkspaceInvalidationChange = "created" | "modified" | "removed" | "unknown";

/**
 * How much of the workspace one notice is about.
 *
 * The cost of acting on these rises steeply: `path` refreshes one row and its parent, `workspace`
 * refreshes everything open. A producer sends the most specific one it can justify, so a broad
 * notice can be read as "observation was genuinely lost" rather than as somebody's shortcut.
 */
export type WorkspaceInvalidationScope = "path" | "directory" | "workspace";

export interface WorkspaceInvalidationNotice {
  sessionId: string;
  source: WorkspaceInvalidationSource;
  scope: WorkspaceInvalidationScope;
  /** Workspace-relative, with forward slashes. Absent exactly when the scope is `workspace`. */
  relativePath?: string;
  /** Present only alongside a `path` scope. */
  change?: WorkspaceInvalidationChange;
  /**
   * Monotonic per session, from 1. A gap is the only evidence available that a notice was lost,
   * and without it a silent channel and a quiet workspace look identical.
   */
  sequence: number;
  occurredAt: string;
  /**
   * How many further observations this notice stands in for. Absent when it stands only for
   * itself — absent rather than zero, because "one change" and "a burst that collapsed" are
   * different facts and zero would read as the first while meaning either.
   */
  coalesced?: number;
}
