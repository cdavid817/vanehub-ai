export interface BoundedResult<T> {
  items: T[];
  truncated: boolean;
  nextCursor: string | null;
}

export type WorkspaceAvailability = "available" | "unavailable";

export interface SessionWorkspaceContext {
  availability: WorkspaceAvailability;
  rootName: string | null;
  reason: string | null;
}

export type DirectoryEntryKind = "directory" | "file";

export interface DirectoryEntry {
  name: string;
  path: string;
  kind: DirectoryEntryKind;
  size: number | null;
}

export interface DirectoryListing extends BoundedResult<DirectoryEntry> {
  context: SessionWorkspaceContext;
  path: string;
}

export type DocumentKind = "markdown" | "text";

export interface SessionDocument {
  name: string;
  path: string;
  kind: DocumentKind;
}

export interface DocumentListing extends BoundedResult<SessionDocument> {
  context: SessionWorkspaceContext;
}

export interface FileSearchMatch {
  name: string;
  path: string;
}

// Deliberately not a BoundedResult: candidate search ranks and caps rather than paginating,
// so there is no cursor to hand back.
export interface FileSearchListing {
  context: SessionWorkspaceContext;
  items: FileSearchMatch[];
  truncated: boolean;
}

export type FileContentStatus = "text" | "binary" | "oversized" | "missing";

export interface FileContent {
  path: string;
  name: string;
  status: FileContentStatus;
  size: number;
  content: string | null;
}

export type GitChangeKind =
  | "unmodified"
  | "modified"
  | "added"
  | "deleted"
  | "renamed"
  | "copied"
  | "untracked"
  | "conflicted";

export interface GitStatusEntry {
  path: string;
  previousPath: string | null;
  index: GitChangeKind;
  worktree: GitChangeKind;
}

export interface GitStatusResult extends BoundedResult<GitStatusEntry> {
  context: SessionWorkspaceContext;
  isGit: boolean;
  branch: string | null;
}

export type GitDiffSource = "working" | "staged";
export type GitDiffLineKind = "context" | "addition" | "deletion";

export interface GitDiffLine {
  kind: GitDiffLineKind;
  content: string;
  oldLineNumber: number | null;
  newLineNumber: number | null;
}

export interface GitDiffHunk {
  header: string;
  oldStart: number;
  oldLines: number;
  newStart: number;
  newLines: number;
  lines: GitDiffLine[];
}

export interface GitDiffFile {
  oldPath: string | null;
  newPath: string;
  binary: boolean;
  oversized: boolean;
  hunks: GitDiffHunk[];
}

export interface GitDiffResult {
  context: SessionWorkspaceContext;
  source: GitDiffSource;
  files: GitDiffFile[];
  truncated: boolean;
}

export type SessionLogLevel = "error" | "warn" | "info" | "debug";

export interface SessionLogEntry {
  id: string;
  timestamp: string;
  level: SessionLogLevel;
  category: string;
  message: string;
  context: Record<string, string>;
}

/**
 * The correlations a reader can narrow logs by.
 *
 * Every one is absent by default and matches only records that carry it. A record emitted without
 * a run is not attributed to whichever run happens to be selected — the alternative would make a
 * filter look like it found evidence of something it merely failed to exclude.
 */
export interface SessionLogCorrelationFilters {
  seatId?: string | null;
  runId?: string | null;
  traceId?: string | null;
  spanId?: string | null;
  operationId?: string | null;
  agentId?: string | null;
}

export interface SessionLogQuery extends SessionLogCorrelationFilters {
  sessionId: string;
  levels: SessionLogLevel[];
  search: string;
  cursor?: string | null;
  limit?: number;
}

/**
 * How much of the corpus the answer actually covers.
 *
 * Carried on the page rather than fetched separately, so a reader cannot end up looking at rows
 * from one moment and a coverage claim from another. `complete` is the only value that licenses a
 * conclusion from an absence, which is why the index is careful about giving it.
 */
export type SessionLogCoverageState = "complete" | "indexing" | "partial" | "unavailable";

export interface SessionLogCoverage {
  state: SessionLogCoverageState;
  oldestAvailableAt?: string;
  newestAvailableAt?: string;
  /** The newest record the index has caught up to. Behind `newestAvailableAt` while indexing. */
  indexedThrough?: string;
  droppedCount: number;
  truncated: boolean;
  /** Stable codes, never prose: a reader groups by them and free text does not group. */
  reasonCodes: string[];
}

export interface SessionLogPage extends BoundedResult<SessionLogEntry> {
  /**
   * Optional so a runtime that predates it still type-checks. A page without one is read as
   * `unavailable` rather than as `complete`: a coverage nobody reported must not be the one value
   * that lets a reader conclude something from an empty list.
   */
  coverage?: SessionLogCoverage;
}

export type SessionLogExportStatus = "exported" | "cancelled" | "unavailable";

export interface SessionLogExportResult {
  status: SessionLogExportStatus;
  path: string | null;
}

export type ShellRuntimeKind = "native" | "remote" | "simulated" | "unavailable";

/**
 * What a Session Shell can actually do, rather than what its label suggests. Capabilities are
 * carried per variant so a caller cannot offer resize to a simulated shell or reconnect to a PTY:
 * the previous string union let the UI ask for both and let the native `remote` value cross a
 * boundary that claimed it could not exist.
 */
export type ShellRuntimeDescriptor =
  | {
      kind: "native";
      supportsResize: true;
      supportsReplay: true;
      supportsReconnect: false;
    }
  | {
      kind: "remote";
      connectionId: string;
      profileRevision: number;
      supportsResize: true;
      supportsReplay: true;
      supportsReconnect: boolean;
    }
  | {
      kind: "simulated";
      supportsResize: false;
      supportsReplay: true;
      supportsReconnect: false;
    }
  | {
      kind: "unavailable";
      reasonCode: string;
      remediation?: string;
    };
