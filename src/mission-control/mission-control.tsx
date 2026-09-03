import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { AgentRegistryEntry } from "../types/agent";
import type {
  MissionControlCounts, MissionControlFacet, MissionControlNavigationTarget,
  MissionControlOverview, MissionControlRunDetail,
} from "../types/mission-control";
import { MissionControlDetailPanel } from "./mission-control-detail-panel";
import {
  clearMissionControlFilters, defaultMissionControlFilterState, MISSION_CONTROL_COUNT_STATES,
  sameStateSet, toMissionControlQuery, type MissionControlFilterState,
} from "./mission-control-query";
import { mergeMissionControlOverview } from "./mission-control-run-precedence";
import { MissionControlRunList, type MissionControlRunListSection } from "./mission-control-run-list";
import {
  applyMissionControlSavedView, captureMissionControlSavedView, readMissionControlSavedViews,
  writeMissionControlSavedViews, type MissionControlSavedView,
} from "./mission-control-saved-views";
import { MissionControlSummary } from "./mission-control-summary";
import { MissionControlToolbar } from "./mission-control-toolbar";
import {
  readMissionControlScrollTop, readMissionControlViewState, writeMissionControlScrollTop,
  writeMissionControlViewState,
} from "./mission-control-view-state";
import { useMissionControlActions } from "./use-mission-control-actions";
import { useMissionControlPolling } from "./use-mission-control-polling";

export function MissionControl({
  agents = [],
  initialRunId,
  onNavigate,
  section,
}: {
  agents?: AgentRegistryEntry[];
  /** 4.8: the run selected the last time this view was left, restored on the way back in. */
  initialRunId?: string;
  onNavigate?: (target: MissionControlNavigationTarget, sourceRunId: string) => void;
  /** 16.2: forwarded to `MissionControlRunList` verbatim -- see its own prop doc comment. */
  section?: MissionControlRunListSection;
}) {
  const { t } = useTranslation();
  const [savedView] = useState(readMissionControlViewState);
  const [overview, setOverview] = useState<MissionControlOverview | null>(null);
  const [selected, setSelected] = useState<MissionControlRunDetail | null>(null);
  const [filter, setFilter] = useState<MissionControlFilterState>(savedView ?? defaultMissionControlFilterState);
  const [cursor, setCursor] = useState<string | null>(null);
  const [activeFacet, setActiveFacet] = useState<MissionControlFacet>("overview");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [savedViews, setSavedViews] = useState<MissionControlSavedView[]>(() => readMissionControlSavedViews());
  const listRef = useRef<HTMLDivElement | null>(null);
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  // 4.9: guards against an earlier inspect() call's response landing after a later one already
  // resolved — otherwise clicking run A then quickly clicking run B could show A's evidence under
  // B's row, or clobber B's already-loaded detail, if A's fetch happens to finish last.
  const latestInspectedRunId = useRef<string | null>(null);

  // Every filter-changing interaction (Toolbar fields, a metric card, Saved Views, Clear) goes
  // through this one function so cursor-reset stays a single, unmissable rule rather than something
  // each caller has to remember on its own.
  const updateFilter = useCallback((patch: Partial<MissionControlFilterState>) => {
    setFilter((current) => ({ ...current, ...patch }));
    setCursor(null);
  }, []);

  // 16.16: returns whether the fetch changed anything observable, so the polling hook's own bounded
  // backoff can widen on a no-op reconcile and reset the instant something real changes.
  const load = useCallback(async (): Promise<boolean> => {
    setLoading(true);
    let changed = true;
    try {
      const fresh = await agentService.getMissionControlOverview(toMissionControlQuery(filter, cursor));
      // 16.15: never let a slow poll response regress a run this page already knows to be newer
      // (e.g. terminal after a just-applied action) -- see mission-control-run-precedence.ts.
      setOverview((current) => {
        const merged = mergeMissionControlOverview(current, fresh);
        changed = JSON.stringify(current) !== JSON.stringify(merged);
        return merged;
      });
      setError(null);
    } catch { setError(t("missionControl.loadError")); changed = true; } finally { setLoading(false); }
    return changed;
  }, [cursor, filter, t]);

  useEffect(() => { void load(); }, [load]);
  // 16.16: stop-while-hidden/offline, bounded backoff, and online/offline reconnect reconciliation
  // -- see use-mission-control-polling.ts's own doc comment for what replaced the previous flat
  // `setInterval` here and why coalesced backend events are not attempted in this same pass.
  const { reconcileNow } = useMissionControlPolling(load);
  useEffect(() => { writeMissionControlViewState(filter); }, [filter]);

  const inspect = useCallback(async (runId: string) => {
    latestInspectedRunId.current = runId;
    try {
      const detail = await agentService.getMissionControlRun(runId);
      if (latestInspectedRunId.current !== runId) return;
      setSelected(detail);
      setActiveFacet("overview");
    } catch {
      if (latestInspectedRunId.current !== runId) return;
      setError(t("missionControl.loadError"));
    }
  }, [t]);
  // 4.8: restores the run selected when this view was last left, on the way back in — mount-only,
  // since RunsDestination fully remounts this component on every destination/tab switch (a fresh
  // `initialRunId` never arrives without a fresh mount to go with it).
  useEffect(() => { if (initialRunId) void inspect(initialRunId); }, [inspect, initialRunId]);
  // 4.8: restores the run-list scroll position once there is actual content to scroll to — doing
  // this before `overview` arrives would restore against an empty, still-collapsing list.
  useEffect(() => {
    if (!overview || !listRef.current) return;
    listRef.current.scrollTop = readMissionControlScrollTop();
    // Deliberately once, the first time real content exists — not on every `overview` refresh,
    // which would fight a reader who has since scrolled on their own.
    // eslint-disable-next-line react-hooks/exhaustive-deps -- see comment above
  }, [Boolean(overview)]);

  // 16.14-16.15: state-aware actions/target-local pending/conflict reconciliation, all in one
  // place -- see the hook's own doc comment for why (registry choice, reconcile-only, conflict
  // detection, terminal-state precedence).
  const { act, mutations } = useMissionControlActions({ onNavigate, setOverview, setSelected });

  // 16.4: a second click on an already-active count clears it back to "all" rather than being a
  // no-op re-apply -- the pressed metric card doubles as its own clear affordance.
  function toggleCount(key: keyof MissionControlCounts) {
    const mapped = MISSION_CONTROL_COUNT_STATES[key];
    updateFilter({ states: sameStateSet(filter.states, mapped) ? [] : mapped });
  }

  function applySavedView(view: MissionControlSavedView) {
    updateFilter(applyMissionControlSavedView(view));
  }
  function saveCurrentView(name: string) {
    const next = [...savedViews, captureMissionControlSavedView(filter, name, crypto.randomUUID())];
    setSavedViews(next);
    writeMissionControlSavedViews(next);
  }
  function deleteSavedView(id: string) {
    const next = savedViews.filter((view) => view.id !== id);
    setSavedViews(next);
    writeMissionControlSavedViews(next);
  }

  return <div className="ucd-panel flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg" data-testid="mission-control">
    <header className="flex flex-wrap items-center gap-2 border-b border-border p-3">
      <div className="min-w-48 flex-1"><h1 className="text-sm font-semibold">{t("missionControl.title")}</h1><p className="text-xs text-muted-foreground">{t("missionControl.description")}</p></div>
    </header>
    <div className="border-b border-border p-3">
      <MissionControlToolbar
        agents={agents}
        filter={filter}
        loading={loading}
        onApplySavedView={applySavedView}
        onClearFilters={() => updateFilter(clearMissionControlFilters(filter))}
        onDeleteSavedView={deleteSavedView}
        onFilterChange={updateFilter}
        onRefresh={reconcileNow}
        onSaveCurrentView={saveCurrentView}
        savedViews={savedViews}
        searchInputRef={searchInputRef}
      />
    </div>
    {error ? <p aria-live="polite" className="m-3 rounded-md border border-destructive/40 bg-destructive/10 p-2 text-xs text-destructive">{error}</p> : null}
    {overview?.counts ? <MissionControlSummary counts={overview.counts} onToggle={toggleCount} states={filter.states} /> : null}
    <div className="grid min-h-0 flex-1 grid-cols-1 overflow-hidden min-[900px]:grid-cols-[minmax(0,1.4fr)_minmax(280px,1fr)]">
      <MissionControlRunList
        agents={agents}
        listRef={listRef}
        loading={loading}
        mutations={mutations.registry}
        onAct={act}
        onDismissError={(run) => mutations.clear(run.runId)}
        onInspect={(run) => void inspect(run.runId)}
        onNextPage={(next) => setCursor(next)}
        onScroll={writeMissionControlScrollTop}
        overview={overview}
        section={section}
      />
      <aside className="min-h-0 overflow-y-auto border-t border-border p-3 min-[900px]:border-l min-[900px]:border-t-0">
        <h2 className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("missionControl.detail")}</h2>
        <MissionControlDetailPanel
          activeFacet={activeFacet}
          agents={agents}
          mutation={selected ? mutations.get(selected.run.runId) : undefined}
          onAct={act}
          onDismissError={() => { if (selected) mutations.clear(selected.run.runId); }}
          onInspect={(run) => void inspect(run.runId)}
          onSelectFacet={setActiveFacet}
          section={section}
          selected={selected}
        />
      </aside>
    </div>
  </div>;
}
