import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { LazyFeature, type LazyFeatureLoader } from "../components/lazy-feature";
import { cn } from "../lib/utils";
import type { AgentRegistryEntry } from "../types/agent";
import type { LoopInspectionTarget } from "../types/loop";
import type { MissionControlNavigationTarget } from "../types/mission-control";
import { shouldRenderPage, type PageLifecyclePolicy } from "../ui/page-lifecycle/page-lifecycle-policy";
import type { RunsSection } from "./workbench-route";

const TABS: { section: RunsSection["section"]; labelKey: string }[] = [
  { section: "attention", labelKey: "missionControl.attention" },
  { section: "active", labelKey: "missionControl.active" },
  { section: "history", labelKey: "missionControl.recent" },
  { section: "loops", labelKey: "layout.activityBar.loops" },
  { section: "schedules", labelKey: "layout.activityBar.scheduledTasks" },
];

type MissionControlProps = { initialRunId?: string; onNavigate?: (target: MissionControlNavigationTarget, sourceRunId: string) => void };
const loadMissionControl: LazyFeatureLoader<MissionControlProps> = () => import("../mission-control/mission-control")
  .then((module) => ({ default: module.MissionControl }));
type LoopCenterProps = { onInspect?: (target: LoopInspectionTarget) => void };
const loadLoopCenter: LazyFeatureLoader<LoopCenterProps> = () => import("../loop-center/loop-center")
  .then((module) => ({ default: module.LoopCenter }));
type ScheduledTasksPanelProps = { agents: AgentRegistryEntry[] };
const loadScheduledTasksPanel: LazyFeatureLoader<ScheduledTasksPanelProps> = () => import("./scheduled-tasks-panel")
  .then((module) => ({ default: module.ScheduledTasksPanel }));

export interface RunsDestinationProps {
  location: RunsSection;
  onSectionChange: (section: RunsSection) => void;
  agents: AgentRegistryEntry[];
  onMissionControlNavigate: (target: MissionControlNavigationTarget, sourceRunId: string) => void;
  onInspectLoop?: (target: LoopInspectionTarget) => void;
}

/**
 * 5.13: Loops and Schedules each hold an in-progress draft a reader would be annoyed to lose —
 * `LoopRunControls`' continue-with-feedback textarea, and `ScheduledTasksPanel`'s inline
 * new-task form. Same classification Settings' three draft-only pages already use
 * (settings-page-lifecycle.ts), same reason: real, currently-uncommitted typed text with no
 * server-side draft of its own. Attention/Active/History are not included — no draft state was
 * found there (Mission Control's own persistence is 4.8's sessionStorage mechanism instead).
 */
const RUNS_TAB_DRAFT_RETENTION: PageLifecyclePolicy = {
  keepAlive: "draft-only", suspendWhenHidden: true, refreshOnFocus: false, backgroundUpdates: "none",
};

/**
 * Attention/Active/History share one `MissionControl` render: it already shows all three as
 * parallel sections rather than exclusive tabs (confirmed by reading the component directly), so
 * there is no per-section view to route between yet — that split is real content work for a
 * later milestone, not something this shell can fake. `definitionId`/`loopRunId`/`scheduleId`
 * deep-linking is likewise not implemented: neither LoopCenter nor ScheduledTasksPanel accept an
 * initial-selection prop today. MissionControl's `runId` is the one exception (4.8): it restores
 * the run last selected before navigating away to an evidence surface and back.
 *
 * Each section keeps its own `LazyFeature` chunk rather than being statically imported here, so
 * navigating within Runs does not pull in code for a section the reader has not opened yet — same
 * chunk boundaries as before this file existed, not a regression from merging them into one.
 *
 * Loops and Schedules stay mounted (CSS `hidden`, not unmounted) once visited, so a draft typed
 * into either survives switching to a different Runs tab and back — the same `shouldRenderPage`
 * mechanism Settings uses, scoped to this destination's own tabs rather than the top-level route.
 * This does not cover leaving Runs entirely: `RunsDestination` itself still fully unmounts on a
 * destination switch (main-layout.tsx's outer ternary, unchanged), a documented, separate boundary
 * (5.11's boundary note) rather than an oversight here.
 */
export function RunsDestination({ location, onSectionChange, agents, onMissionControlNavigate, onInspectLoop }: RunsDestinationProps) {
  const { t } = useTranslation();
  const [visitedSections, setVisitedSections] = useState<Set<RunsSection["section"]>>(() => new Set([location.section]));
  useEffect(() => {
    setVisitedSections((current) => (current.has(location.section) ? current : new Set(current).add(location.section)));
  }, [location.section]);

  const loopsActive = location.section === "loops";
  const schedulesActive = location.section === "schedules";

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex shrink-0 items-center gap-1 border-b border-border-subtle px-2 py-1.5" role="tablist">
        {TABS.map((tab) => (
          <button
            aria-selected={location.section === tab.section}
            className={cn(
              "rounded-md px-2.5 py-1.5 text-sm",
              location.section === tab.section ? "bg-nav-active-soft text-primary" : "text-muted-foreground hover:bg-accent",
            )}
            key={tab.section}
            onClick={() => onSectionChange({ section: tab.section })}
            role="tab"
            type="button"
          >
            {t(tab.labelKey)}
          </button>
        ))}
      </div>
      <div className="min-h-0 flex-1">
        {shouldRenderPage(RUNS_TAB_DRAFT_RETENTION, loopsActive, visitedSections.has("loops")) ? (
          <div className="h-full min-h-0" hidden={!loopsActive}>
            <LazyFeature className="h-full min-h-0" componentProps={{ onInspect: onInspectLoop }} loader={loadLoopCenter} />
          </div>
        ) : null}
        {shouldRenderPage(RUNS_TAB_DRAFT_RETENTION, schedulesActive, visitedSections.has("schedules")) ? (
          <div className="h-full min-h-0" hidden={!schedulesActive}>
            <LazyFeature className="h-full min-h-0" componentProps={{ agents }} loader={loadScheduledTasksPanel} />
          </div>
        ) : null}
        {location.section === "attention" || location.section === "active" || location.section === "history" ? (
          <LazyFeature className="h-full min-h-0" componentProps={{ initialRunId: location.runId, onNavigate: onMissionControlNavigate }} loader={loadMissionControl} />
        ) : null}
      </div>
    </div>
  );
}
