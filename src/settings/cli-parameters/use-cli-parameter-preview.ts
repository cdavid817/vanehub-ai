import { useEffect, useRef, useState } from "react";
import { agentService } from "../../services/runtime-agent-client";
import type { ManagedCliAgentId } from "../../types/agent";
import type { CliLaunchScope, CliParameterSelections } from "../../types/cli-parameter";
import type { CliParameterPreview } from "../../types/cli-parameter-profile";

export interface CliParameterPreviewState {
  /** The last preview that succeeded. It is kept across a refresh and across a failure, so the
   * panel never blanks while the user is still typing. */
  preview: CliParameterPreview | null;
  refreshing: boolean;
  /** True while `preview` describes an older draft than the one on screen. */
  stale: boolean;
  error: unknown;
}

const debounceMs = 200;

/**
 * Debounced, read-only preview with latest-request-wins.
 *
 * Every request carries an identity. A response whose identity is not the newest one is ignored
 * rather than raced into state, which is the only reliable way to stop a slow request for an
 * abandoned draft from overwriting a fast one for the current draft.
 */
export function useCliParameterPreview(
  agentId: ManagedCliAgentId | null,
  catalogVersion: string,
  scope: CliLaunchScope,
  selections: CliParameterSelections,
  enabled: boolean,
): CliParameterPreviewState {
  const [state, setState] = useState<CliParameterPreviewState>({
    preview: null,
    refreshing: false,
    stale: false,
    error: null,
  });
  const latest = useRef(0);
  const key = `${agentId ?? ""}|${scope}|${catalogVersion}|${JSON.stringify(selections)}`;

  useEffect(() => {
    if (!agentId || !enabled) return;
    const requestId = latest.current + 1;
    latest.current = requestId;
    setState((current) => ({ ...current, refreshing: true, stale: current.preview !== null }));

    const timer = setTimeout(() => {
      void agentService
        .previewCliParameterProfile({
          agentId,
          catalogVersion,
          scope,
          selections,
          requestId: String(requestId),
        })
        .then((preview) => {
          if (latest.current !== requestId) return;
          setState({ preview, refreshing: false, stale: false, error: null });
        })
        .catch((error: unknown) => {
          if (latest.current !== requestId) return;
          // The previous preview stays on screen: a rejected draft is a reason to explain, not a
          // reason to erase the last thing that worked.
          setState((current) => ({ ...current, refreshing: false, stale: true, error }));
        });
    }, debounceMs);

    return () => {
      clearTimeout(timer);
    };
    // `key` collapses the draft into one dependency; the individual values are read inside.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, enabled]);

  return state;
}
