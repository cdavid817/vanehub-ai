import { useCallback, useRef, useState } from "react";
import { agentService } from "../services/runtime-agent-client";
import { conciseError } from "./create-session-dialog-utils";
import { createInspectionTracker } from "./create-session-inspection";
import { normalizeDisplayPath } from "../lib/session-path";
import type { ProjectInspection } from "../types/agent";

/**
 * The selected project folder and what is known about it.
 *
 * Owns the path as well as the inspection because the two are one fact, and splitting them is what
 * lets them disagree. An inspection answers a question about one specific path, and two can be in
 * flight at once: the user types, changes their mind, types again. They do not return in the order
 * they were asked — the second is often on a warm cache while the first is still walking a cold
 * share.
 *
 * Two independent things invalidate a result, so both are checked:
 *
 * - the request id, which rules out a superseded inspection;
 * - the path, which rules out a result for a folder that is no longer selected. The id alone
 *   cannot catch this, because the field updates as the user types and only inspects on blur, so
 *   the path can move on without any new request being made.
 *
 * The path is tracked in a ref updated *synchronously* alongside the state. Reading it from a
 * rendered value would compare against whatever React had committed by the time the promise
 * settled, and a fast inspection can resolve before its own path change has rendered — rejecting
 * its own perfectly good answer.
 *
 * Logical cancellation, not transport cancellation. The in-flight call still completes; its answer
 * is not applied. Claiming it was aborted would assert something about the native side that
 * nothing here can know.
 */
export function useProjectInspection({
  onError,
  onFolderChanged,
  t,
}: {
  onError: (message: string | null) => void;
  /**
   * Called whenever a different folder is inspected, for state derived from the old one.
   *
   * Inside the hook rather than at each call site, because there are two entry points — typing a
   * path and browsing for one — and a caller that wired only the first would leave a worktree
   * request attached to a folder the user replaced through the second.
   */
  onFolderChanged: () => void;
  t: (key: string) => string;
}) {
  const [projectPath, setProjectPathState] = useState("");
  const [inspection, setInspection] = useState<ProjectInspection | null>(null);
  const tracker = useRef(createInspectionTracker());
  const latestPath = useRef("");

  /** Sets both halves at once, so no reader can observe them disagreeing. */
  const setProjectPath = useCallback((value: string) => {
    latestPath.current = value;
    setProjectPathState(value);
  }, []);

  const inspectPath = useCallback(
    async (path: string) => {
      const trimmed = normalizeDisplayPath(path.trim());
      setProjectPath(trimmed);
      // Both entry points pass through here, so anything derived from the previous folder is
      // cleared once rather than at each call site.
      onFolderChanged();
      setInspection(null);
      onError(null);
      if (!trimmed) return;
      const ticket = tracker.current.begin(trimmed);
      try {
        const result = await agentService.inspectProject(trimmed);
        if (tracker.current.accepts(ticket, latestPath.current)) setInspection(result);
      } catch (inspectionError) {
        if (tracker.current.accepts(ticket, latestPath.current)) {
          onError(conciseError(inspectionError, t));
        }
      }
    },
    [onError, onFolderChanged, setProjectPath, t],
  );

  const browseProject = useCallback(async () => {
    onError(null);
    try {
      const selected = await agentService.selectProjectDirectory();
      if (selected) await inspectPath(selected);
    } catch (browseError) {
      onError(conciseError(browseError, t));
    }
  }, [inspectPath, onError, t]);

  /** Clears the selection and invalidates anything in flight. For open, close, or mode change. */
  const resetInspection = useCallback(() => {
    tracker.current.reset();
    setProjectPath("");
    setInspection(null);
  }, [setProjectPath]);

  return {
    browseProject,
    inspectPath,
    inspection,
    projectPath,
    resetInspection,
    setProjectPath,
  };
}
