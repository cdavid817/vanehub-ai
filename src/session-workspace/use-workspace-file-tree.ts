import { useCallback, useEffect, useMemo, useState } from "react";
import { useQueries } from "@tanstack/react-query";
import { agentService } from "../services/runtime-agent-client";
import type { DirectoryEntry } from "../types/session-workspace";
import { collectDirectoryPages } from "./directory-pagination";
import { workspaceErrorKey, type WorkspaceErrorKey } from "./workspace-error";
import { workspaceQueryKeys } from "./workspace-query-keys";

export interface TreeRow {
  entry: DirectoryEntry;
  depth: number;
}

export function flattenFileRows(
  entriesByPath: Record<string, DirectoryEntry[]>,
  expanded: ReadonlySet<string>,
): TreeRow[] {
  const result: TreeRow[] = [];
  const visit = (parent: string, depth: number) => {
    for (const entry of entriesByPath[parent] ?? []) {
      result.push({ entry, depth });
      if (entry.kind === "directory" && expanded.has(entry.path)) visit(entry.path, depth + 1);
    }
  };
  visit("", 0);
  return result;
}

/**
 * One query per open directory.
 *
 * The version this replaces loaded subdirectories imperatively into component state, which made a
 * targeted refresh impossible: there was nothing to invalidate, so the only way to pick up a change
 * anywhere was to rebuild the whole tree and lose every expanded folder. A key per directory is what
 * lets a notice about `src/main.rs` refresh `src` and leave the other twelve open folders alone.
 *
 * Expanding no longer waits for the listing. The folder opens, its rows arrive when they arrive, and
 * a directory that fails to load leaves the rest of the tree standing — which is what a reader
 * needs when one folder is on a disconnected network share and the others are not.
 */
export function useWorkspaceFileTree(sessionId: string | null, isVisible: boolean) {
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(() => new Set());

  // A new session is a different tree. Keeping the expansion would open paths that may not exist
  // there, and each one would be a listing request for a directory nobody asked about.
  useEffect(() => {
    setExpanded(new Set());
  }, [sessionId]);

  const openDirectories = useMemo(() => ["", ...[...expanded].sort()], [expanded]);

  const listings = useQueries({
    queries: openDirectories.map((path) => ({
      // Disabled rather than unmounted while hidden: the listing stays in the cache and on screen,
      // and the tab stops re-reading a directory nobody is looking at.
      enabled: Boolean(sessionId) && isVisible,
      queryKey: workspaceQueryKeys.directory(sessionId ?? "", path),
      queryFn: () =>
        collectDirectoryPages((cursor) =>
          agentService.listSessionDirectory(sessionId ?? "", path, cursor),
        ),
    })),
  });

  const entriesByPath = useMemo(() => {
    const result: Record<string, DirectoryEntry[]> = {};
    openDirectories.forEach((path, index) => {
      const items = listings[index]?.data?.items;
      if (items) result[path] = items;
    });
    return result;
  }, [listings, openDirectories]);

  const rows = useMemo(() => flattenFileRows(entriesByPath, expanded), [entriesByPath, expanded]);

  /** Directories that are open but could not be read. */
  const failedPaths = useMemo(() => {
    const result = new Set<string>();
    openDirectories.forEach((path, index) => {
      if (listings[index]?.error) result.add(path);
    });
    return result;
  }, [listings, openDirectories]);

  const retryers = useMemo(() => {
    const result = new Map<string, () => void>();
    openDirectories.forEach((path, index) => {
      const listing = listings[index];
      if (listing) result.set(path, () => void listing.refetch());
    });
    return result;
  }, [listings, openDirectories]);

  const toggleDirectory = useCallback(
    (path: string) => {
      // A folder that is open and failed retries instead of collapsing. Collapsing would take the
      // reader two clicks to get back to the same failure, and the thing they wanted was another
      // attempt.
      if (expanded.has(path) && failedPaths.has(path)) {
        retryers.get(path)?.();
        return;
      }
      setExpanded((current) => {
        const next = new Set(current);
        if (!next.delete(path)) next.add(path);
        return next;
      });
    },
    [expanded, failedPaths, retryers],
  );

  /**
   * Whether a directory is showing its contents.
   *
   * Not the same as being expanded. One that is expanded and failed has nothing under it, and a
   * chevron pointing down over no rows reads as an empty folder — which is a different fact from
   * one that could not be read, and the wrong one.
   */
  const isOpen = useCallback(
    (path: string) => expanded.has(path) && !failedPaths.has(path),
    [expanded, failedPaths],
  );

  /**
   * Opens every directory along a path.
   *
   * Quick Open lands a reader somewhere they have not expanded to, and revealing only the leaf's
   * parent would leave a row with no visible ancestors — present in the tree and unreachable by
   * scrolling to it.
   */
  const revealDirectory = useCallback((path: string) => {
    if (!path) return;
    setExpanded((current) => {
      const next = new Set(current);
      const segments = path.split("/");
      for (let index = 0; index < segments.length; index += 1) {
        next.add(segments.slice(0, index + 1).join("/"));
      }
      return next;
    });
  }, []);

  const rootListing = listings[0];
  // The first failure, not a collected list. A reader acts on one message, and a tree with three
  // unreachable folders has one cause far more often than three.
  const failure = listings.find((listing) => listing.error);

  return {
    entriesByPath,
    expanded,
    isOpen,
    revealDirectory,
    rows,
    toggleDirectory,
    error: failure ? workspaceErrorKey(failure.error) : (null as WorkspaceErrorKey | null),
    isLoading: Boolean(rootListing?.isLoading),
    /** Any open directory being cut short, because any of them can make the tree incomplete. */
    truncated: listings.some((listing) => listing.data?.truncated),
    /**
     * The first reason a listing gave for not being whole, if one did.
     *
     * Separate from `truncated`, which only says another page exists. A directory that was refused
     * or whose scan stopped early is not one more page away from complete, and telling a reader to
     * scroll for the rest would send them looking for entries nothing is going to produce.
     */
    incompleteReason:
      listings
        .map((listing) => listing.data?.coverage)
        .find((coverage) => coverage && coverage.state !== "complete")?.reasonCode ?? null,
    hasRoot: Boolean(entriesByPath[""]),
  };
}
