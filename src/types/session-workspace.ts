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

export interface SessionLogQuery {
  sessionId: string;
  levels: SessionLogLevel[];
  search: string;
  /**
   * Absent means every seat in the session. A concrete seat matches only records that carry that
   * seat correlation; a record emitted without one is not silently attributed to the selection.
   */
  seatId?: string | null;
  cursor?: string | null;
  limit?: number;
}

export type SessionLogPage = BoundedResult<SessionLogEntry>;

export type SessionLogExportStatus = "exported" | "cancelled" | "unavailable";

export interface SessionLogExportResult {
  status: SessionLogExportStatus;
  path: string | null;
}

export type ShellConnectionState = "connecting" | "connected" | "disconnected" | "failed";

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

export interface ShellSession {
  shellId: string;
  sessionId: string;
  state: ShellConnectionState;
  runtime: ShellRuntimeDescriptor;
}

export interface CreateShellInput {
  sessionId: string;
  rows: number;
  cols: number;
  /** Which participant owns this shell. Absent in a single-seat session, which has only one. */
  seatId?: string | null;
}

export interface ResizeShellInput {
  shellId: string;
  rows: number;
  cols: number;
}

export type ShellEvent =
  | { type: "output"; shellId: string; sessionId: string; content: string }
  | { type: "state"; shellId: string; sessionId: string; state: ShellConnectionState; error?: string };
