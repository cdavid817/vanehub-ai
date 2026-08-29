import type { WorkspaceInvalidationNotice } from "../types/session-workspace-inspection";
import { workspacePathSegment, workspaceQueryKeys } from "./workspace-query-keys";

/**
 * Which queries one change notice makes stale.
 *
 * Pure, and separate from the hook that applies it, because this is the part with the interesting
 * mistakes in it. Every one of them is silent: too broad and the tree collapses expanded folders
 * and loses the reader's place on every agent write; too narrow and a panel keeps showing content
 * that is no longer there. Neither raises anything.
 *
 * The rule underneath all of it: refresh what the notice actually implicates, and nothing else.
 */

/** The directory a path sits in. `""` for a top-level entry, which is the root's own key. */
export function parentDirectoryOf(relativePath: string): string {
  const separator = relativePath.lastIndexOf("/");
  return separator < 0 ? "" : relativePath.slice(0, separator);
}

/**
 * Whether a path is inside a directory, or is that directory.
 *
 * The separator check is the whole function. Without it `src-generated` is inside `src`, which is a
 * real collapse that reads like a typo: two sibling folders, one silently refreshed by the other's
 * changes, and no error anywhere.
 */
export function isWithinDirectory(directory: string, candidate: string): boolean {
  if (directory === "") return true;
  return candidate === directory || candidate.startsWith(`${directory}/`);
}

/**
 * One invalidation instruction.
 *
 * `pathWithin` exists because prefix matching runs out of precision at exactly one place: a path is
 * the last segment of its key, so there is no prefix that means "every preview under `src`". Rather
 * than round that up to "every preview", the filter carries the directory and the caller narrows.
 */
export interface WorkspaceInvalidationFilter {
  readonly queryKey: readonly unknown[];
  readonly pathWithin?: string;
}

/** Whether one cached query key is what a filter meant. */
export function filterMatchesKey(
  filter: WorkspaceInvalidationFilter,
  queryKey: readonly unknown[],
): boolean {
  if (filter.pathWithin === undefined) return true;
  const path = workspacePathSegment(queryKey);
  return path !== null && isWithinDirectory(filter.pathWithin, path);
}

/**
 * What to refresh for one notice.
 *
 * Git status and the review are in every case on purpose. A file changing is exactly what moves a
 * repository between clean and dirty, and a Changes tab that stayed clean after an agent edited a
 * tracked file is wrong in the way a reader acts on — they conclude nothing happened.
 */
export function invalidationFiltersFor(
  notice: WorkspaceInvalidationNotice,
): readonly WorkspaceInvalidationFilter[] {
  const { sessionId } = notice;

  if (notice.scope === "workspace") {
    // Observation was lost, so nothing narrower can be justified. Still scoped to the session:
    // another session's panels saw nothing happen, and refreshing them would spend a second
    // workspace's reads on this one's uncertainty.
    return [{ queryKey: workspaceQueryKeys.session(sessionId) }];
  }

  const relativePath = notice.relativePath ?? "";
  const shared: WorkspaceInvalidationFilter[] = [
    { queryKey: workspaceQueryKeys.gitStatus(sessionId) },
    { queryKey: workspaceQueryKeys.review(sessionId) },
    // Every search result is a claim about which files contain something, and any write can make or
    // break any of them. Nothing here can tell which without re-running the search.
    { queryKey: workspaceQueryKeys.searches(sessionId) },
    // Documents are collected across the tree, so a change anywhere can add or remove one.
    { queryKey: workspaceQueryKeys.documents(sessionId) },
  ];

  if (notice.scope === "directory") {
    return [
      // The directory's own listing, not its parent's: its entries changed, it did not.
      { queryKey: workspaceQueryKeys.directory(sessionId, relativePath) },
      // Entries appearing and disappearing includes renames, which can replace a file somebody has
      // open. Narrowed to this directory rather than refetching every preview in the session.
      { queryKey: workspaceQueryKeys.previews(sessionId), pathWithin: relativePath },
      { queryKey: workspaceQueryKeys.gitDiffs(sessionId), pathWithin: relativePath },
      ...shared,
    ];
  }

  return [
    // The parent, because that is the listing this path appears in. Refreshing the path's own
    // listing instead would refresh a directory only when the path happened to be one.
    { queryKey: workspaceQueryKeys.directory(sessionId, parentDirectoryOf(relativePath)) },
    // And its own, in case it is a directory whose contents moved with it. Harmless when it is a
    // file: nothing is cached under that key, so nothing refetches.
    { queryKey: workspaceQueryKeys.directory(sessionId, relativePath) },
    { queryKey: workspaceQueryKeys.preview(sessionId, relativePath) },
    // A write is exactly the event that changes what is retained about a file, so the two refresh
    // together. Leaving this out would show a stale count beside fresh content.
    { queryKey: workspaceQueryKeys.fileEvidence(sessionId, relativePath) },
    { queryKey: workspaceQueryKeys.gitDiffsFor(sessionId, relativePath) },
    ...shared,
  ];
}

/**
 * Whether a selection is still a thing that exists.
 *
 * Derived from the refreshed listing rather than from the notice that triggered it. A notice says
 * something was removed; a listing says what is there — and the second answers the question for
 * every cause, including an explicit refresh and a change nobody was told about.
 *
 * `undefined` entries mean the parent has not answered yet, which is not evidence of absence. That
 * distinction is what keeps a selection from being dropped during its own refetch, when the reader
 * is still looking at the file and nothing has actually happened to it.
 */
export function selectionStillExists(
  selectedPath: string | null,
  parentEntries: readonly { readonly path: string }[] | undefined,
): boolean {
  if (!selectedPath) return true;
  if (!parentEntries) return true;
  return parentEntries.some((entry) => entry.path === selectedPath);
}
