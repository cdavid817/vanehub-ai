import { ChevronDown, ChevronRight, Columns3, List, RefreshCw } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { LazyFeature, type LazyFeatureLoader } from "../components/lazy-feature";
import { cn } from "../lib/utils";
import type { InboxView } from "../main-layout/workspace-route";
import type { ActivityNavigator } from "../system-activity/activity-navigation";
import type { MissionControlNavigationTarget } from "../types/mission-control";
import { readInboxViewMode, rememberInboxViewMode, type InboxViewMode } from "./inbox-feed";
import { InboxSections } from "./inbox-sections";
import { useInboxFeed } from "./use-inbox-feed";

const loadWorkBoard: LazyFeatureLoader<Record<string, never>> = () => import("../work-board/work-board")
  .then((module) => ({ default: module.WorkBoard }));
type MissionControlProps = { onNavigate?: (target: MissionControlNavigationTarget) => void };
const loadMissionControl: LazyFeatureLoader<MissionControlProps> = () => import("../mission-control/mission-control")
  .then((module) => ({ default: module.MissionControl }));
type SystemActivityProps = { maintenanceControls: boolean; onNavigate?: ActivityNavigator };
const loadSystemActivity: LazyFeatureLoader<SystemActivityProps> = () => import("../system-activity/system-activity-view")
  .then((module) => ({ default: module.SystemActivityView }));

type InboxDisclosure = "console" | "log";

export interface InboxProps {
  /** Route view; `board` is the Board mode over the Needs attention section. */
  view: InboxView;
  active?: boolean;
  onViewChange: (view: InboxView) => void;
  onNavigate?: (target: MissionControlNavigationTarget) => void;
  onNavigateActivity?: ActivityNavigator;
  /** The badge Inbox computed from its own feed, so the shell need not fetch it again. */
  onBadgeChange?: (count: number) => void;
}

const toggleClass = "ucd-interactive flex h-8 items-center gap-1 rounded-md border px-2 text-xs";

/** Opened on demand and then kept mounted, so reopening does not replay a loading state. */
function Disclosure({ children, id, onToggle, open, title }: { children: ReactNode; id: string; onToggle: () => void; open: boolean; title: string }) {
  const [visited, setVisited] = useState(open);
  useEffect(() => { if (open) setVisited(true); }, [open]);
  return (
    <section className="border-t border-border" data-testid={id}>
      <button aria-controls={`${id}-region`} aria-expanded={open} className="flex h-9 w-full items-center gap-2 px-3 text-left text-xs font-semibold" onClick={onToggle} type="button">
        {open ? <ChevronDown aria-hidden="true" className="h-3.5 w-3.5" /> : <ChevronRight aria-hidden="true" className="h-3.5 w-3.5" />}
        {title}
      </button>
      <div className={cn("min-h-0 p-2", open ? "flex h-[28rem] max-h-[60vh] flex-col" : "hidden")} hidden={!open} id={`${id}-region`} role="region">
        {visited ? children : null}
      </div>
    </section>
  );
}

export function Inbox({ active = true, onBadgeChange, onNavigate, onNavigateActivity = () => undefined, onViewChange, view }: InboxProps) {
  const { t } = useTranslation();
  const [storedMode, setStoredMode] = useState<InboxViewMode>(readInboxViewMode);
  // The route can force Board (a `/workspace/work-board` redirect does); otherwise the persisted
  // preference decides, so the toggle survives a relaunch like the other layout preferences.
  const mode: InboxViewMode = view === "board" ? "board" : storedMode;
  // The list is the only consumer of the feed, so Board mode stops the polling with it.
  const model = useInboxFeed({ active: active && mode === "list", onNavigate });
  const [boardVisited, setBoardVisited] = useState(mode === "board");
  // One disclosure at a time: two open together outgrow the panel and the second gets clipped.
  const [openDisclosure, setOpenDisclosure] = useState<InboxDisclosure | null>(null);
  useEffect(() => {
    rememberInboxViewMode(mode);
    if (mode === "board") setBoardVisited(true);
  }, [mode]);
  useEffect(() => {
    if (!model.loading) onBadgeChange?.(model.badge);
  }, [model.badge, model.loading, onBadgeChange]);

  function selectMode(next: InboxViewMode) {
    setStoredMode(next);
    rememberInboxViewMode(next);
    onViewChange(next === "board" ? "board" : "attention");
  }
  const toggleDisclosure = (target: InboxDisclosure) => setOpenDisclosure((current) => current === target ? null : target);
  const focusedView: InboxView = view === "board" ? "attention" : view;

  return (
    <div className="ucd-panel flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg" data-inbox-mode={mode} data-testid="inbox">
      <header className="flex flex-wrap items-center gap-2 border-b border-border p-3">
        <div className="min-w-48 flex-1">
          <h1 className="text-sm font-semibold">{t("inbox.title")}</h1>
          <p className="text-xs text-muted-foreground">{t("inbox.description")}</p>
        </div>
        <div className="flex items-center gap-2">
          <div aria-label={t("inbox.view.label")} className="flex items-center gap-1" role="group">
            <button aria-pressed={mode === "list"} className={cn(toggleClass, mode === "list" ? "border-primary bg-[hsl(var(--nav-active-soft))] text-primary" : "border-input")} data-testid="inbox-view-list" onClick={() => selectMode("list")} type="button">
              <List aria-hidden="true" className="h-3.5 w-3.5" />{t("inbox.view.list")}
            </button>
            <button aria-pressed={mode === "board"} className={cn(toggleClass, mode === "board" ? "border-primary bg-[hsl(var(--nav-active-soft))] text-primary" : "border-input")} data-testid="inbox-view-board" onClick={() => selectMode("board")} type="button">
              <Columns3 aria-hidden="true" className="h-3.5 w-3.5" />{t("inbox.view.board")}
            </button>
          </div>
          <button aria-label={t("inbox.refresh")} className="ucd-interactive grid h-8 w-8 place-items-center rounded-md border border-input" data-testid="inbox-refresh" onClick={model.refresh} title={t("inbox.refresh")} type="button">
            <RefreshCw aria-hidden="true" className={cn("h-4 w-4", (model.loading || model.refreshing) && "animate-spin")} />
          </button>
        </div>
      </header>
      {model.error ? <p aria-live="polite" className="m-3 rounded-md border border-destructive/40 bg-destructive/10 p-2 text-xs text-destructive">{model.error}</p> : null}
      <div className={cn("min-h-0 flex-1", mode === "board" ? "flex p-2" : "hidden")} data-testid="inbox-board">
        {boardVisited ? <LazyFeature className="h-full min-h-0 flex-1" componentProps={{}} loader={loadWorkBoard} /> : null}
      </div>
      {mode === "list" ? (
        <InboxSections
          focusedView={focusedView}
          loading={model.loading}
          onAct={(run, action) => void model.act(run, action)}
          onInspect={() => setOpenDisclosure("console")}
          onNavigateActivity={onNavigateActivity}
          sections={model.sections}
        />
      ) : null}
      {mode === "list" ? (
        <>
          <Disclosure id="inbox-console" onToggle={() => toggleDisclosure("console")} open={openDisclosure === "console"} title={t("inbox.console")}>
            <LazyFeature className="min-h-0 flex-1" componentProps={{ onNavigate }} loader={loadMissionControl} />
          </Disclosure>
          <Disclosure id="inbox-activity-log" onToggle={() => toggleDisclosure("log")} open={openDisclosure === "log"} title={t("inbox.activityLog")}>
            <LazyFeature className="min-h-0 flex-1" componentProps={{ maintenanceControls: false, onNavigate: onNavigateActivity }} loader={loadSystemActivity} />
          </Disclosure>
        </>
      ) : null}
    </div>
  );
}
