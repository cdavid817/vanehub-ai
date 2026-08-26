import type {
  DirectoryListing,
  DocumentListing,
  FileContent,
  FileSearchListing,
  GitDiffResult,
  GitDiffSource,
  GitStatusResult,
} from "../types/session-workspace";
import type {
  FileEvidenceLinks,
  WorkspaceInspectionCapabilities,
  WorkspaceInvalidationNotice,
  WorkspaceContentSearchResult,
  WorkspacePathSearchResult,
} from "../types/session-workspace-inspection";

export interface WorkspaceContentSearchInput {
  sessionId: string;
  query: string;
  /**
   * Chosen by the caller so the same caller can cancel it.
   *
   * An id this side returned would arrive with the answer, which is exactly too late to stop the
   * search that produced it.
   */
  searchId: string;
  limit?: number;
}

export interface WorkspacePathSearchInput {
  sessionId: string;
  query: string;
  /** From a previous page. A cursor issued for another query is refused rather than applied. */
  cursor?: string;
  limit?: number;
}

/**
 * Reading a session's workspace, wherever it is.
 *
 * Split out of `AgentService` rather than shortened in place: these methods are one subject and
 * they are still growing, and the interface they were sharing had reached the line rule. A group
 * that keeps arriving together belongs in a file of its own, where the next method has somewhere
 * obvious to go.
 *
 * Both runtime clients implement the whole of it. The Web adapter answers from fixtures and says
 * `simulated` when asked what it is, which is the honest answer for a build with no workspace at
 * all — a demo claiming to read this machine sends a reader looking for files that do not exist.
 */
export interface SessionWorkspaceInspectionService {
  /**
   * What this session's workspace can be asked, before anything asks it.
   *
   * Read first by every panel that inspects a workspace: a remote host missing a prerequisite is
   * an ordinary situation, and finding out one failed call at a time turns it into five errors.
   */
  getWorkspaceInspectionCapabilities(sessionId: string): Promise<WorkspaceInspectionCapabilities>;

  /**
   * Live notice that something in a workspace changed.
   *
   * One subscription for every session rather than one per session: the panels a reader has open
   * change as they navigate, and a per-session subscription would be torn down and rebuilt on each
   * switch — losing whatever was published in the gap, with nothing to say a notice went missing.
   */
  subscribeWorkspaceInvalidation(
    handler: (notice: WorkspaceInvalidationNotice) => void,
  ): Promise<() => void>;

  /**
   * Quick Open: relative paths matching what was typed, ranked and paged.
   *
   * Distinct from `searchSessionFiles`, which ranks prompt-mention candidates and therefore
   * filters to source extensions and skips directories. Widening that one would have changed what
   * an `@` offers, which is a different feature.
   */
  searchWorkspacePaths(input: WorkspacePathSearchInput): Promise<WorkspacePathSearchResult>;

  /**
   * Content search: positions inside files, bounded and interruptible.
   *
   * Fixed-string and case-insensitive on both providers. Not because patterns would be less useful,
   * but because the two implementations are different engines and a pattern language is the one
   * thing two engines cannot be made to agree about.
   */
  searchWorkspaceContent(
    input: WorkspaceContentSearchInput,
  ): Promise<WorkspaceContentSearchResult>;

  /**
   * Stops a running content search.
   *
   * Resolves to whether one was still running. A caller cannot know whether their cancel beat the
   * search's own completion, so `false` means "nothing to stop", not "you did something wrong".
   */
  cancelWorkspaceSearch(searchId: string): Promise<boolean>;
  /**
   * What is retained about one file, so a panel knows whether to offer a link to it.
   *
   * Takes a workspace-relative path; the digest the journal stores is computed on the native side
   * from the session's own root. A caller cannot ask about a file in somebody else's workspace
   * because there is nowhere to supply one.
   */
  getFileEvidenceLinks(sessionId: string, relativePath: string): Promise<FileEvidenceLinks>;
  listSessionDirectory(sessionId: string, path?: string): Promise<DirectoryListing>;
  readSessionFile(sessionId: string, path: string): Promise<FileContent>;
  listSessionDocuments(sessionId: string): Promise<DocumentListing>;
  searchSessionFiles(sessionId: string, query: string, maxResults?: number): Promise<FileSearchListing>;
  getSessionGitStatus(sessionId: string): Promise<GitStatusResult>;
  getSessionGitDiff(sessionId: string, path: string, source: GitDiffSource): Promise<GitDiffResult>;
}
