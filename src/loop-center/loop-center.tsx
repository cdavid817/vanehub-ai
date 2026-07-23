import { forwardRef, useEffect, useRef, useState, type Dispatch, type ReactNode, type RefObject, type SetStateAction } from "react";
import { PanelLeftOpen, PanelRightOpen } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useLoopDefinitionsQuery, useLoopRunQuery, useLoopRunsQuery } from "../hooks/use-loop-queries";
import { LoopInspector } from "./loop-inspector";
import { LoopDefinitionDialog } from "./loop-definition-dialog";
import { LoopNavigation } from "./loop-navigation";
import { LoopTimeline } from "./loop-timeline";
import type { LoopInspectionTarget } from "../types/loop";

export function LoopCenter({ onInspect }: { onInspect?: (target: LoopInspectionTarget) => void }) {
  const { t } = useTranslation();
  const [definitionId, setDefinitionId] = useState<string | null>(null);
  const [runId, setRunId] = useState<string | null>(null);
  const [navigationOpen, setNavigationOpen] = useState(false);
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [editorDefinitionId, setEditorDefinitionId] = useState<string | "new" | null>(null);
  const navigationRef = useRef<HTMLElement>(null);
  const inspectorRef = useRef<HTMLElement>(null);
  const navigationTriggerRef = useRef<HTMLButtonElement>(null);
  const inspectorTriggerRef = useRef<HTMLButtonElement>(null);
  const definitions = useLoopDefinitionsQuery();
  const runs = useLoopRunsQuery(definitionId ?? undefined);
  const run = useLoopRunQuery(runId);

  useEffect(() => {
    const available = definitions.data ?? [];
    if (!definitionId || !available.some((item) => item.id === definitionId)) {
      setDefinitionId(available[0]?.id ?? null);
    }
  }, [definitionId, definitions.data]);

  useEffect(() => {
    const available = runs.data ?? [];
    if (!runId || !available.some((item) => item.id === runId)) {
      setRunId(available[0]?.id ?? null);
    }
  }, [runId, runs.data]);

  const error = definitions.error ?? runs.error ?? run.error;
  useDrawerFocus(navigationOpen, setNavigationOpen, navigationRef, navigationTriggerRef);
  useDrawerFocus(inspectorOpen, setInspectorOpen, inspectorRef, inspectorTriggerRef);
  const closeDrawers = () => { setNavigationOpen(false); setInspectorOpen(false); };

  return (
    <div className="ucd-panel relative grid h-full min-h-0 w-full min-w-0 grid-cols-1 overflow-hidden rounded-lg min-[1024px]:grid-cols-[minmax(220px,280px)_minmax(360px,1fr)_minmax(260px,340px)]">
      {navigationOpen || inspectorOpen ? <button aria-label={t("loops.drawers.close")} className="absolute inset-0 z-30 bg-background/70 backdrop-blur-[1px] min-[1024px]:hidden" onClick={closeDrawers} title={t("loops.drawers.close")} type="button" /> : null}
      <LoopNavigation
        className={`absolute inset-y-0 left-0 z-40 w-[min(88vw,320px)] border-r border-border/70 shadow-xl transition-transform duration-200 min-[1024px]:static min-[1024px]:w-auto min-[1024px]:translate-x-0 min-[1024px]:shadow-none ${navigationOpen ? "translate-x-0" : "-translate-x-full invisible min-[1024px]:visible"}`}
        definitions={definitions.data ?? []}
        id="loop-navigation-drawer"
        loading={definitions.isLoading || runs.isLoading}
        onClose={() => setNavigationOpen(false)}
        onCreateDefinition={() => setEditorDefinitionId("new")}
        onDefinitionChange={(id) => { setDefinitionId(id); setRunId(null); setNavigationOpen(false); }}
        onEditDefinition={() => { if (definitionId) setEditorDefinitionId(definitionId); }}
        onRunChange={(id) => { setRunId(id); setNavigationOpen(false); }}
        ref={navigationRef}
        runs={runs.data ?? []}
        selectedDefinitionId={definitionId}
        selectedRunId={runId}
      />
      <div className="flex min-h-0 min-w-0 flex-col border-border/70 min-[1024px]:border-x" role="main">
        <div className="flex h-11 shrink-0 items-center justify-between border-b border-border/70 px-2 min-[1024px]:hidden">
          <IconButton controls="loop-navigation-drawer" label={t("loops.navigation.open")} onClick={() => { setInspectorOpen(false); setNavigationOpen(true); }} open={navigationOpen} ref={navigationTriggerRef}><PanelLeftOpen aria-hidden="true" className="h-4 w-4" /></IconButton>
          <span className="truncate px-2 text-xs font-semibold">{run.data?.definitionSnapshot.name ?? t("loops.title")}</span>
          <IconButton controls="loop-inspector-drawer" label={t("loops.inspector.open")} onClick={() => { setNavigationOpen(false); setInspectorOpen(true); }} open={inspectorOpen} ref={inspectorTriggerRef}><PanelRightOpen aria-hidden="true" className="h-4 w-4" /></IconButton>
        </div>
        <div className="min-h-0 min-w-0 flex-1 overflow-y-auto p-3 sm:p-4">
          {error ? <StateMessage title={t("loops.states.error")} value={error instanceof Error ? error.message : String(error)} /> : null}
          {!error && (definitions.isLoading || runs.isLoading) ? <StateMessage title={t("loops.states.loading")} /> : null}
          {!error && !definitions.isLoading && definitions.data?.length === 0 ? <StateMessage title={t("loops.states.emptyDefinitions")} /> : null}
          {!error && definitions.data?.length !== 0 && !runs.isLoading && runs.data?.length === 0 ? <StateMessage title={t("loops.states.emptyRuns")} /> : null}
          {!error && runId && run.data ? <LoopTimeline onInspect={onInspect} refreshing={run.isFetching} run={run.data} /> : null}
        </div>
      </div>
      <LoopInspector className={`absolute inset-y-0 right-0 z-40 w-[min(88vw,340px)] border-l border-border/70 shadow-xl transition-transform duration-200 min-[1024px]:static min-[1024px]:w-auto min-[1024px]:translate-x-0 min-[1024px]:shadow-none ${inspectorOpen ? "translate-x-0" : "translate-x-full invisible min-[1024px]:visible"}`} id="loop-inspector-drawer" loading={run.isLoading} onClose={() => setInspectorOpen(false)} onInspect={onInspect} ref={inspectorRef} run={run.data ?? null} />
      {editorDefinitionId ? (
        <LoopDefinitionDialog
          definition={editorDefinitionId === "new" ? null : definitions.data?.find((item) => item.id === editorDefinitionId) ?? null}
          onClose={() => setEditorDefinitionId(null)}
          onSaved={(saved, startedRunId) => {
            setEditorDefinitionId(null);
            setDefinitionId(saved.id);
            setRunId(startedRunId);
            void definitions.refetch();
            void runs.refetch();
          }}
        />
      ) : null}
    </div>
  );
}

const IconButton = forwardRef<HTMLButtonElement, { children: ReactNode; controls: string; label: string; onClick: () => void; open: boolean }>(function IconButton({ children, controls, label, onClick, open }, ref) {
  return <button aria-controls={controls} aria-expanded={open} aria-label={label} className="grid h-8 w-8 shrink-0 place-items-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" onClick={onClick} ref={ref} title={label} type="button">{children}</button>;
});

function useDrawerFocus(open: boolean, setOpen: Dispatch<SetStateAction<boolean>>, drawerRef: RefObject<HTMLElement>, triggerRef: RefObject<HTMLButtonElement>) {
  const wasOpen = useRef(false);
  useEffect(() => {
    if (!open) {
      if (wasOpen.current) triggerRef.current?.focus();
      wasOpen.current = false;
      return;
    }
    wasOpen.current = true;
    const drawer = drawerRef.current;
    if (!drawer) return;
    const focusable = () => Array.from(drawer.querySelectorAll<HTMLElement>('button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'));
    (focusable()[0] ?? drawer).focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") { event.preventDefault(); setOpen(false); triggerRef.current?.focus(); return; }
      if (event.key !== "Tab") return;
      const items = focusable();
      if (items.length === 0) { event.preventDefault(); drawer.focus(); return; }
      const first = items[0];
      const last = items[items.length - 1];
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
      if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [drawerRef, open, setOpen, triggerRef]);
}

function StateMessage({ title, value }: { title: string; value?: string }) {
  return (
    <div className="flex h-full min-h-48 flex-col items-center justify-center gap-2 text-center">
      <p className="text-sm font-medium text-foreground">{title}</p>
      {value ? <p className="max-w-md text-xs text-destructive">{value}</p> : null}
    </div>
  );
}
