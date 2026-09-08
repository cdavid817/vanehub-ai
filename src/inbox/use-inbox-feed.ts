import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { createMissionControlCoalescer } from "../mission-control/event-coalescer";
import { agentService } from "../services/runtime-agent-client";
import type { MissionControlAction, MissionControlNavigationTarget, MissionControlRunSummary } from "../types/mission-control";
import { classifyInboxFeed, countInboxBadge, emptyInboxFeed, type InboxFeed, type InboxSections } from "./inbox-feed";
import { loadInboxFeed } from "./load-inbox-feed";

export interface InboxFeedModel {
  feed: InboxFeed;
  sections: InboxSections;
  badge: number;
  /** True until the first load settles; background polls never set it again. */
  loading: boolean;
  /** True while a refresh the user asked for is in flight. */
  refreshing: boolean;
  error: string | null;
  refresh: () => void;
  /** The same action contract Mission Control rows expose. */
  act: (run: MissionControlRunSummary, action: MissionControlAction) => Promise<void>;
}

interface InboxFeedOptions {
  /** Polling and reconciliation only run while the surface is on screen. */
  active: boolean;
  onNavigate?: (target: MissionControlNavigationTarget) => void;
}

/**
 * Mission Control's polling cadence, and its coalescer for the receipts an action produces: a
 * burst of state changes from one click collapses into a single reload instead of one per event.
 * Each poll hands the previous feed back to the loader so unchanged activity is not refetched.
 */
export function useInboxFeed({ active, onNavigate }: InboxFeedOptions): InboxFeedModel {
  const { t } = useTranslation();
  const [feed, setFeed] = useState<InboxFeed>(emptyInboxFeed);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const requestRef = useRef(0);
  const feedRef = useRef(feed);
  feedRef.current = feed;

  const load = useCallback(async () => {
    const request = requestRef.current + 1;
    requestRef.current = request;
    const { feed: next, failed } = await loadInboxFeed(feedRef.current);
    // A stale response must not overwrite a newer one.
    if (requestRef.current !== request) return;
    setFeed(next);
    setError(failed ? t("inbox.loadError") : null);
    setLoading(false);
  }, [t]);

  const refresh = useCallback(() => {
    setRefreshing(true);
    void load().finally(() => setRefreshing(false));
  }, [load]);

  const coalescer = useMemo(() => createMissionControlCoalescer(() => { void load(); }), [load]);
  useEffect(() => () => coalescer.dispose(), [coalescer]);

  useEffect(() => {
    if (!active) return undefined;
    void load();
    const reconcile = () => { if (document.visibilityState === "visible") void load(); };
    const polling = window.setInterval(reconcile, 2_000);
    window.addEventListener("focus", reconcile);
    document.addEventListener("visibilitychange", reconcile);
    return () => {
      window.clearInterval(polling);
      window.removeEventListener("focus", reconcile);
      document.removeEventListener("visibilitychange", reconcile);
    };
  }, [active, load]);

  const act = useCallback(async (run: MissionControlRunSummary, action: MissionControlAction) => {
    if (action === "open" || action === "approval" || action === "review") {
      if (run.navigation) onNavigate?.(run.navigation);
      return;
    }
    try {
      const receipt = await agentService.performMissionControlAction({ runId: run.runId, version: run.version, action });
      coalescer.push({ runId: receipt.run.runId, version: receipt.run.version, kind: "state" });
    } catch {
      setError(t("missionControl.actionError"));
    }
  }, [coalescer, onNavigate, t]);

  const sections = useMemo(() => classifyInboxFeed(feed), [feed]);
  const badge = useMemo(() => countInboxBadge(feed), [feed]);
  return { feed, sections, badge, loading, refreshing, error, refresh, act };
}
