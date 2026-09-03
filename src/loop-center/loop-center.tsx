import { forwardRef, useEffect, useState, type ReactNode } from "react";
import { PanelLeftOpen, PanelRightOpen, Plus, Repeat2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { useLoopDefinitionsQuery, useLoopRunQuery, useLoopRunsQuery } from "../hooks/use-loop-queries";
import { LoopDefinitionDialog } from "./loop-definition-dialog";
import { LoopDefinitionOverview } from "./loop-definition-overview";
import { LoopPreflightDialog } from "./loop-preflight-dialog";
import { LoopTimeline } from "./loop-timeline";
import { DestinationLayout } from "../ui/destination-layout/DestinationLayout";
import type { LayoutTier } from "../ui/destination-layout/use-layout-tier";
import type { LoopDefinition, LoopInspectionTarget } from "../types/loop";
import { LOOP_INSPECTOR_PANE_BOUNDS, LOOP_NAVIGATION_PANE_BOUNDS, useLoopInspection, useLoopInspectorRegion, useLoopNavigationRegion } from "./loop-center-regions";

export interface LoopCenterProps {
  onInspect?: (target: LoopInspectionTarget) => void;
  /** 17.2: the route's own current selection (`RunsSection`'s `definitionId`/`loopRunId`,
   *  workbench-route.ts) -- the first real consumer of these fields (parsed by `parseRunsSection`
   *  since 17.1 but never read; see runs-destination.tsx's prior audit note). Mirrors
   *  `ScheduledTasksPanel`'s `scheduleId` (19.3) and `GoalCenter`'s `goalId` (15.1): optional so
   *  this component still works standalone (tests, or the responsive/states suites that render it
   *  with no props at all) without a routed parent. */
  definitionId?: string;
  loopRunId?: string;
  /**
   * Reports every definition/run selection change back as one combined shape rather than two
   * separate callbacks (`onSelectDefinition`/`onSelectRun`): `RunsSection` already models the pair
   * as one unit (`{definitionId?, loopRunId?}`), and a run belongs to exactly one definition
   * (`selectDefinition` below always clears it in the same call), so the route never observes an
   * intermediate call where the two ids momentarily disagree.
   */
  onSelectionChange?: (selection: { definitionId?: string; loopRunId?: string }) => void;
}

export function LoopCenter({ definitionId, loopRunId, onInspect, onSelectionChange }: LoopCenterProps) {
  const { t } = useTranslation();
  const [selectedDefinitionId, setSelectedDefinitionId] = useState<string | null>(definitionId ?? null);
  const [selectedRunId, setSelectedRunId] = useState<string | null>(loopRunId ?? null);
  const [navigationOpen, setNavigationOpen] = useState(false);
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [navigationWidth, setNavigationWidth] = useState(LOOP_NAVIGATION_PANE_BOUNDS.default);
  const [inspectorWidth, setInspectorWidth] = useState(LOOP_INSPECTOR_PANE_BOUNDS.default);
  const [tier, setTier] = useState<LayoutTier>("wide");
  const [editorDefinitionId, setEditorDefinitionId] = useState<string | "new" | null>(null);
  const [preflightDefinition, setPreflightDefinition] = useState<LoopDefinition | null>(null);
  const definitions = useLoopDefinitionsQuery();
  const runs = useLoopRunsQuery(selectedDefinitionId ?? undefined);
  const run = useLoopRunQuery(selectedRunId);
  const selectedDefinition = definitions.data?.find((item) => item.id === selectedDefinitionId) ?? null;

  /** Selecting a definition always clears any run selected under a previously selected one -- a
   *  run belongs to exactly one definition, so carrying a stale run selection across a definition
   *  switch would show a run that does not belong to what is now selected. Reports the combined
   *  result back in one call (see `onSelectionChange` above) so the route is never told about the
   *  two ids separately. */
  function selectDefinition(id: string | null) {
    setSelectedDefinitionId(id);
    setSelectedRunId(null);
    onSelectionChange?.({ definitionId: id ?? undefined, loopRunId: undefined });
  }

  function selectRun(id: string | null) {
    setSelectedRunId(id);
    onSelectionChange?.({ definitionId: selectedDefinitionId ?? undefined, loopRunId: id ?? undefined });
  }

  useEffect(() => {
    const available = definitions.data ?? [];
    if (!selectedDefinitionId || !available.some((item) => item.id === selectedDefinitionId)) {
      setSelectedDefinitionId(available[0]?.id ?? null);
    }
  }, [selectedDefinitionId, definitions.data]);

  useEffect(() => {
    const available = runs.data ?? [];
    if (selectedRunId && !available.some((item) => item.id === selectedRunId)) setSelectedRunId(null);
  }, [selectedRunId, runs.data]);

  // 17.2: restores the route's own current selection when it changes while this stays mounted
  // (Loops stays mounted across a Runs tab switch, runs-destination.tsx's 5.13 note) -- the same
  // "route drives selection" shape as ScheduledTasksPanel's scheduleId / GoalCenter's goalId.
  // Unlike those single-id precedents, a routed definitionId always carries loopRunId along with
  // it (clearing it when absent) rather than syncing independently, because a run belongs to
  // exactly one definition -- syncing loopRunId on its own here would let a run selected under a
  // previously routed definition survive a route-driven definition switch.
  useEffect(() => {
    if (!definitionId) return;
    setSelectedDefinitionId(definitionId);
    setSelectedRunId(loopRunId ?? null);
  }, [definitionId, loopRunId]);

  const error = definitions.error ?? runs.error ?? run.error;
  const inspection = useLoopInspection({ onInspect, run: run.data ?? null, runId: selectedRunId });

  const navigationRegion = useLoopNavigationRegion({
    definitions: definitions.data ?? [],
    loading: definitions.isLoading || runs.isLoading,
    onCreateDefinition: () => setEditorDefinitionId("new"),
    onDefinitionChange: (id) => { selectDefinition(id); setNavigationOpen(false); },
    onEditDefinition: () => { if (selectedDefinitionId) setEditorDefinitionId(selectedDefinitionId); },
    onOpenChange: setNavigationOpen,
    onRunChange: (id) => { selectRun(id); setNavigationOpen(false); },
    onWidthChange: setNavigationWidth,
    open: navigationOpen,
    runs: runs.data ?? [],
    selectedDefinitionId,
    selectedRunId,
    tier,
    width: navigationWidth,
  });
  const inspectorRegion = useLoopInspectorRegion({
    inspection,
    onOpenChange: setInspectorOpen,
    onWidthChange: setInspectorWidth,
    open: inspectorOpen,
    tier,
    width: inspectorWidth,
  });

  return (
    <div className="h-full min-h-0 w-full min-w-0" data-testid="loop-center">
      <DestinationLayout
        inspector={inspectorRegion}
        main={(
          <div className="ucd-panel flex h-full min-h-0 min-w-0 flex-col overflow-hidden rounded-lg" role="main">
            {tier !== "wide" ? (
              <div className="flex h-11 shrink-0 items-center justify-between border-b border-border/70 px-2">
                {/* Standard tier keeps navigation inline (DestinationLayoutBody's own
                    `navigationInline`) -- only compact/narrow need a trigger for it. */}
                {tier !== "standard" ? (
                  <IconButton controls="loop-navigation-drawer" label={t("loops.navigation.open")} onClick={() => setNavigationOpen(true)} open={navigationOpen}>
                    <PanelLeftOpen aria-hidden="true" className="h-4 w-4" />
                  </IconButton>
                ) : null}
                <span className="truncate px-2 text-xs font-semibold">{run.data?.definitionSnapshot.name ?? t("loops.title")}</span>
                <IconButton controls="loop-inspector-drawer" label={t("loops.inspector.open")} onClick={() => setInspectorOpen(true)} open={inspectorOpen}>
                  <PanelRightOpen aria-hidden="true" className="h-4 w-4" />
                </IconButton>
              </div>
            ) : null}
            <div className="min-h-0 min-w-0 flex-1 overflow-y-auto p-3 sm:p-4">
              {error ? <StateMessage title={t("loops.states.error")} value={error instanceof Error ? error.message : String(error)} /> : null}
              {!error && (definitions.isLoading || runs.isLoading) ? <StateMessage title={t("loops.states.loading")} /> : null}
              {!error && !definitions.isLoading && definitions.data?.length === 0 ? (
                <EmptyDefinitions onCreate={() => setEditorDefinitionId("new")} />
              ) : null}
              {/* No `definitions.refetch()` here: `useDeleteLoopDefinitionMutation`'s own `onSuccess`
                  (use-loop-mutations.ts) already removes this row from the cache directly (task
                  17.14), and a refetch here would re-introduce the whole-collection reload that patch
                  exists to avoid. */}
              {!error && selectedDefinition && !selectedRunId && !runs.isLoading ? <LoopDefinitionOverview definition={selectedDefinition} onDeleted={() => selectDefinition(null)} onEdit={() => setEditorDefinitionId(selectedDefinition.id)} onPreflight={() => setPreflightDefinition(selectedDefinition)} runs={runs.data ?? []} /> : null}
              {!error && selectedRunId && run.data ? <LoopTimeline onInspect={onInspect} refreshing={run.isFetching} run={run.data} /> : null}
            </div>
          </div>
        )}
        navigation={navigationRegion}
        onTierChange={setTier}
      />
      {editorDefinitionId ? (
        <LoopDefinitionDialog
          definition={editorDefinitionId === "new" ? null : definitions.data?.find((item) => item.id === editorDefinitionId) ?? null}
          onClose={() => setEditorDefinitionId(null)}
          onSaved={(saved, requestStart) => {
            setEditorDefinitionId(null);
            selectDefinition(saved.id);
            if (requestStart) setPreflightDefinition(saved);
            void definitions.refetch();
            void runs.refetch();
          }}
        />
      ) : null}
      {preflightDefinition ? <LoopPreflightDialog definition={preflightDefinition} onClose={() => setPreflightDefinition(null)} onEdit={() => { setPreflightDefinition(null); setEditorDefinitionId(preflightDefinition.id); }} onStarted={(startedRunId) => { setPreflightDefinition(null); selectRun(startedRunId); void runs.refetch(); }} /> : null}
    </div>
  );
}

const IconButton = forwardRef<HTMLButtonElement, { children: ReactNode; controls: string; label: string; onClick: () => void; open: boolean }>(function IconButton({ children, controls, label, onClick, open }, ref) {
  return <button aria-controls={controls} aria-expanded={open} aria-label={label} className="grid h-8 w-8 shrink-0 place-items-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring" onClick={onClick} ref={ref} title={label} type="button">{children}</button>;
});

function StateMessage({ title, value }: { title: string; value?: string }) {
  return (
    <div className="flex h-full min-h-48 flex-col items-center justify-center gap-2 text-center">
      <p className="text-sm font-medium text-foreground">{title}</p>
      {value ? <p className="max-w-md text-xs text-destructive">{value}</p> : null}
    </div>
  );
}

/**
 * The first-run state used to be one centred sentence, with creation reachable only through a
 * 24px icon in the navigation header. Matches the icon/title/explanation/action shape the chat
 * welcome screen and notification centre already use.
 */
function EmptyDefinitions({ onCreate }: { onCreate: () => void }) {
  const { t } = useTranslation();
  return (
    <div className="flex h-full min-h-48 flex-col items-center justify-center gap-3 p-6 text-center">
      <span className="grid h-12 w-12 place-items-center rounded-lg border border-border bg-background">
        <Repeat2 aria-hidden="true" className="h-5 w-5 text-primary" />
      </span>
      <p className="text-sm font-semibold">{t("loops.states.emptyDefinitions")}</p>
      <p className="max-w-sm text-xs leading-5 text-muted-foreground">{t("loops.states.emptyDefinitionsDescription")}</p>
      <Button onClick={onCreate} size="sm" type="button">
        <Plus aria-hidden="true" className="h-3.5 w-3.5" />
        {t("loops.states.emptyDefinitionsAction")}
      </Button>
    </div>
  );
}
