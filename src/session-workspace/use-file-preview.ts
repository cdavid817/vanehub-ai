import { useEffect, useRef } from "react";
import { useQuery } from "@tanstack/react-query";
import { agentService } from "../services/runtime-agent-client";
import type { FileContent } from "../types/session-workspace";
import { workspaceErrorKey, type WorkspaceErrorKey } from "./workspace-error";
import { workspaceQueryKeys } from "./workspace-query-keys";

/**
 * Why the visible preview may not be the answer to the current question.
 *
 * A union rather than two booleans, because the three non-current states are three different
 * sentences and a reader acts differently on each. "This file is being re-read" means what is on
 * screen is probably still right. "You are looking at another file" means it is definitely not the
 * one you asked for. "That file could not be read" means the request is over and this is what is
 * left. Collapsing them into `isStale` would put one message on screen for all three.
 */
export type PreviewStatus =
  | { kind: "current" }
  | { kind: "refreshing" }
  | { kind: "loading"; pendingPath: string }
  | { kind: "failed"; pendingPath: string; reason: WorkspaceErrorKey };

export interface PreviewState {
  /** What to render, which may be an earlier file than the one selected. */
  shown: FileContent | null;
  status: PreviewStatus;
  /** True before anything has ever loaded, when there is genuinely nothing to show. */
  isEmpty: boolean;
}

/**
 * The selected file, and whatever was last successfully read.
 *
 * The rule is that a reader never loses a file they were reading to a request that has not
 * finished. Clicking a second file, an invalidation refetch, and a failed read all leave the last
 * good content on screen — and all three say so, because content that stayed while its label
 * changed would be the same thing as showing the wrong file.
 *
 * Retention is explicit rather than React Query's `keepPreviousData`. That would cover the first
 * two cases and not the third: on failure the query has no data, and the reader would lose the file
 * they were reading because a *different* one could not be read.
 */
export function useFilePreview(sessionId: string | null, selectedPath: string | null): PreviewState {
  const query = useQuery({
    enabled: Boolean(sessionId) && Boolean(selectedPath),
    queryKey: workspaceQueryKeys.preview(sessionId ?? "", selectedPath ?? ""),
    queryFn: () => agentService.readSessionFile(sessionId ?? "", selectedPath ?? ""),
  });

  // A ref rather than state: this is a record of what was rendered, not an input to rendering, and
  // storing it in state would re-render once more on every successful load to no effect.
  //
  // The session travels with it. Clearing the ref in an effect on session change would be too
  // late — an effect that only writes a ref triggers no re-render, so the first render after a
  // switch would still show the previous workspace’s file with nothing to correct it.
  const lastLoaded = useRef<{ sessionId: string; file: FileContent } | null>(null);
  useEffect(() => {
    if (query.data && sessionId) lastLoaded.current = { sessionId, file: query.data };
  }, [query.data, sessionId]);

  if (!selectedPath) {
    return { shown: null, status: { kind: "current" }, isEmpty: true };
  }

  // Ignored outright when it belongs to another session. Showing one workspace’s content under
  // another’s tree is the one retention that is never an improvement.
  const retained =
    lastLoaded.current?.sessionId === sessionId ? lastLoaded.current.file : null;

  if (query.data) {
    return {
      shown: query.data,
      // Fetching with data already present is a refetch: the content is probably still right, and
      // saying "loading" would suggest the panel is empty when it is not.
      status: query.isFetching ? { kind: "refreshing" } : { kind: "current" },
      isEmpty: false,
    };
  }

  if (query.error) {
    return retained
      ? {
          shown: retained,
          status: {
            kind: "failed",
            pendingPath: selectedPath,
            reason: workspaceErrorKey(query.error),
          },
          isEmpty: false,
        }
      : // Nothing to fall back to, so the failure is the whole answer.
        {
          shown: null,
          status: {
            kind: "failed",
            pendingPath: selectedPath,
            reason: workspaceErrorKey(query.error),
          },
          isEmpty: true,
        };
  }

  return retained
    ? { shown: retained, status: { kind: "loading", pendingPath: selectedPath }, isEmpty: false }
    : { shown: null, status: { kind: "loading", pendingPath: selectedPath }, isEmpty: true };
}
