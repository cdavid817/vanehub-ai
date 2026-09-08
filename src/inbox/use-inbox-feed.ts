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
  /** Whether the surface is on screen at all; nothing loads while it is not. */
  active: boolean;
  /** Whether to keep polling; a one-off load still happens when only `active` is true. */
  polling?: boolean;
  onNavigate?: (target: MissionControlNavigationTarget) => void;
}

export const inboxPollIntervalMs = 2_000;

/**
 * Mission Control's polling cadence with one load in flight at a time: a poll that fires while a
 * load is still running is remembered and run once afterwards, so a slow backend never has its
 * finished responses discarded by the next tick. Each load hands the previous feed back to the
 * loader so unchanged activity is not refetched.
 */
export function useInboxFeed({ active, onNavigate, polling = true }: InboxFeedOptions): InboxFeedModel {
  const { t } = useTranslation();
  const [feed, setFeed] = useState<InboxFeed>(emptyInboxFeed);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const feedRef = useRef(feed);
  feedRef.current = feed;
  const inFlightRef = useRef<Promise<void> | null>(null);
  const queuedRef = useRef<{ force: boolean } | null>(null);
  const mountedRef = useRef(true);
  // Set on every mount, not only initialised: StrictMode mounts, unmounts, and mounts again in
  // development, and a flag that only ever went false would make the hook drop every result.
  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  const load = useCallback((options: { force?: boolean } = {}): Promise<void> => {
    if (inFlightRef.current) {
      queuedRef.current = { force: Boolean(queuedRef.current?.force || options.force) };
      return inFlightRef.current;
    }
    const run = (async () => {
      const { feed: next, failed } = await loadInboxFeed(feedRef.current, options);
      if (!mountedRef.current) return;
      setFeed(next);
      setError(failed ? t("inbox.loadError") : null);
      setLoading(false);
    })().catch(() => {
      if (mountedRef.current) setError(t("inbox.loadError"));
    }).finally(() => {
      inFlightRef.current = null;
      const queued = queuedRef.current;
      queuedRef.current = null;
      if (queued && mountedRef.current) void load(queued);
    });
    inFlightRef.current = run;
    return run;
  }, [t]);

  const refresh = useCallback(() => {
    setRefreshing(true);
    void load({ force: true }).finally(() => { if (mountedRef.current) setRefreshing(false); });
  }, [load]);

  const coalescer = useMemo(() => createMissionControlCoalescer(() => { void load(); }), [load]);
  useEffect(() => () => coalescer.dispose(), [coalescer]);

  useEffect(() => {
    if (!active) return undefined;
    void load();
    if (!polling) return undefined;
    const reconcile = () => { if (document.visibilityState === "visible") void load(); };
    const timer = window.setInterval(reconcile, inboxPollIntervalMs);
    window.addEventListener("focus", reconcile);
    document.addEventListener("visibilitychange", reconcile);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener("focus", reconcile);
      document.removeEventListener("visibilitychange", reconcile);
    };
  }, [active, load, polling]);

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
