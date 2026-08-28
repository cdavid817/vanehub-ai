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

/** How a text file was encoded. Two variants, because this application does not transcode. */
export type FileEncoding = "utf-8" | "utf-8-bom";

/** Which line endings a file uses. `mixed` is the one worth surfacing. */
export type FileNewlineStyle = "lf" | "crlf" | "mixed" | "none";

export interface FileContent {
  path: string;
  name: string;
  status: FileContentStatus;
  size: number;
  content: string | null;
  /**
   * Absent for anything that is not text.
   *
   * Absent rather than defaulted: a binary file has no encoding this application established, and
   * naming one would describe a decode that never happened.
   */
  encoding?: FileEncoding;
  /** Absent for anything that is not text. */
  newline?: FileNewlineStyle;
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

/**
 * Which machine a session's workspace is on, and what can be read there.
 *
 * Per-capability rather than one flag, because a remote host with Git but no ripgrep is an
 * ordinary host: a single flag would either hide the search gap or disable the four things that
 * work. The Shell needs none of this and stays reachable in every case.
 */
export type WorkspaceInspectionProviderId = "local" | "ssh" | "simulated";

export type WorkspaceWatchMode = "native" | "polling" | "event-derived" | "none";

export interface WorkspaceCapabilityState {
  available: boolean;
  /** A stable token with a `workspace.capability.reason.*` translation. Never a message. */
  reasonCode?: string;
  /** What would fix it, also as a token. "Unavailable" and "install ripgrep" are different facts. */
  remediation?: string;
}

export interface WorkspaceInspectionCapabilities {
  provider: WorkspaceInspectionProviderId;
  /** Absent for a local workspace, which is what a reader assumes when nothing says otherwise. */
  targetLabel?: string;
  listFiles: WorkspaceCapabilityState;
  readTextFiles: WorkspaceCapabilityState;
  searchFiles: WorkspaceCapabilityState;
  gitStatus: WorkspaceCapabilityState;
  gitDiff: WorkspaceCapabilityState;
  watchMode: WorkspaceWatchMode;
}
