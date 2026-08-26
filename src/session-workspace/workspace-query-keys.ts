/**
 * Query keys for every read the workspace panels make, built in one place.
 *
 * Targeted invalidation is only possible if the keys have a shape somebody can target. A key
 * assembled by hand at each call site is a key nobody can match against later: the tree would use
 * `["session-workspace", "directory", id, path]` and the preview `["preview", id, path]`, and the
 * invalidation router would have to know both spellings and every future one.
 *
 * The prefix order is deliberate — root, family, session, then whatever narrows it further — so a
 * partial key is a meaningful filter. `tree(session)` matches every directory of one session and
 * nothing of another, which is what lets a workspace-wide notice refresh one session's panels
 * without touching a second session the user also has open.
 */

const root = "session-workspace" as const;

/** Families, named once. A string typed twice is a string that will eventually differ. */
export const workspaceQueryFamilies = Object.freeze({
  directory: "directory",
  preview: "preview",
  documents: "documents",
  search: "search",
  gitStatus: "git-status",
  gitDiff: "git-diff",
  review: "review",
  capabilities: "capabilities",
} as const);

export const workspaceQueryKeys = {
  all: () => [root] as const,

  /** Everything belonging to one session, whichever panel reads it. */
  session: (sessionId: string) => [root, sessionId] as const,

  /**
   * One directory listing. The path is the last segment, so `tree(session)` — the prefix without
   * it — matches every open directory of that session.
   */
  directory: (sessionId: string, path: string) =>
    [root, sessionId, workspaceQueryFamilies.directory, path] as const,

  tree: (sessionId: string) => [root, sessionId, workspaceQueryFamilies.directory] as const,

  preview: (sessionId: string, path: string) =>
    [root, sessionId, workspaceQueryFamilies.preview, path] as const,

  previews: (sessionId: string) => [root, sessionId, workspaceQueryFamilies.preview] as const,

  documents: (sessionId: string) => [root, sessionId, workspaceQueryFamilies.documents] as const,

  /**
   * The query text is part of the key: two searches are two results, and sharing an entry between
   * them shows the previous query's matches under the current query's heading.
   */
  search: (sessionId: string, query: string) =>
    [root, sessionId, workspaceQueryFamilies.search, query] as const,

  searches: (sessionId: string) => [root, sessionId, workspaceQueryFamilies.search] as const,

  gitStatus: (sessionId: string) => [root, sessionId, workspaceQueryFamilies.gitStatus] as const,

  /**
   * Source belongs in the key. The staged and unstaged diffs of one file are different content,
   * and a shared entry would show one under the other's label.
   */
  gitDiff: (sessionId: string, path: string, source: string) =>
    [root, sessionId, workspaceQueryFamilies.gitDiff, path, source] as const,

  /** Both sources for one file, as a prefix. */
  gitDiffsFor: (sessionId: string, path: string) =>
    [root, sessionId, workspaceQueryFamilies.gitDiff, path] as const,

  gitDiffs: (sessionId: string) => [root, sessionId, workspaceQueryFamilies.gitDiff] as const,

  review: (sessionId: string) => [root, sessionId, workspaceQueryFamilies.review] as const,

  capabilities: (sessionId: string) =>
    [root, sessionId, workspaceQueryFamilies.capabilities] as const,
} as const;

/**
 * The path a key is about, for the families that carry one.
 *
 * The position is a property of the key layout above, so it is read here rather than at every call
 * site that needs it. A caller that hard-coded index 3 would be a second statement of the layout,
 * and moving a segment would leave it silently reading the wrong one.
 */
export function workspacePathSegment(queryKey: readonly unknown[]): string | null {
  if (queryKey[0] !== root) return null;
  const path = queryKey[3];
  return typeof path === "string" ? path : null;
}
