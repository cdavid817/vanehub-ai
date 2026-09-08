import { CalendarClock, Repeat2, Target, type LucideIcon } from "lucide-react";
import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import { LazyFeature, type LazyFeatureLoader } from "../components/lazy-feature";
import { cn } from "../lib/utils";
import { automationsViews, type AutomationsView } from "../main-layout/workspace-route";
import type { AgentRegistryEntry } from "../types/agent";
import type { LoopInspectionTarget } from "../types/loop";

type LoopCenterProps = { onInspect?: (target: LoopInspectionTarget) => void };
const loadLoopCenter: LazyFeatureLoader<LoopCenterProps> = () => import("../loop-center/loop-center")
  .then((module) => ({ default: module.LoopCenter }));
type ScheduledProps = { active?: boolean; agents: AgentRegistryEntry[]; focusOnActivate?: boolean };
const loadScheduledTasks: LazyFeatureLoader<ScheduledProps> = () => import("./scheduled-tasks-panel")
  .then((module) => ({ default: module.ScheduledTasksPanel }));
const loadGoalCenter: LazyFeatureLoader<Record<string, never>> = () => import("../goal-center/goal-center")
  .then((module) => ({ default: module.GoalCenter }));

export interface AutomationsProps {
  view: AutomationsView;
  active?: boolean;
  agents: AgentRegistryEntry[];
  onViewChange: (view: AutomationsView) => void;
  onInspectLoop?: (target: LoopInspectionTarget) => void;
}

const tabs: Array<{ view: AutomationsView; icon: LucideIcon; labelKey: string; panelId: string }> = [
  // The loops panel keeps the id the Loop Center's section had as a destination, so nothing that
  // addressed it by id has to move.
  { view: "loops", icon: Repeat2, labelKey: "automations.tab.loops", panelId: "loop-center" },
  { view: "scheduled", icon: CalendarClock, labelKey: "automations.tab.scheduled", panelId: "automations-scheduled" },
  { view: "goals", icon: Target, labelKey: "automations.tab.goals", panelId: "automations-goals" },
];

/**
 * A thin tab shell over three surfaces that already existed as destinations. Each tab lazy-loads
 * on first visit and stays mounted afterwards, exactly as the destinations did.
 */
export function Automations({ active = true, agents, onInspectLoop, onViewChange, view }: AutomationsProps) {
  const { t } = useTranslation();
  const [visited, setVisited] = useState<Set<AutomationsView>>(() => new Set(active ? [view] : []));
  // True while the current selection was made with the arrow keys: the tablist keeps focus then,
  // and a hosted surface must not pull it into its first control.
  const [keyboardSelection, setKeyboardSelection] = useState(false);
  const tablistRef = useRef<HTMLDivElement>(null);
  // Only a shown tab counts as visited: while the shell is hidden the view it is handed is a
  // placeholder, and mounting a surface for it would load data nobody asked for.
  useEffect(() => {
    if (!active) return;
    setVisited((current) => current.has(view) ? current : new Set(current).add(view));
  }, [active, view]);
  // Roving focus: when the selection changed while a tab had focus (the arrow keys), focus follows
  // it; a click elsewhere or a route change from outside must not steal focus into the tablist.
  useEffect(() => {
    const tablist = tablistRef.current;
    if (!tablist || !tablist.contains(document.activeElement)) return;
    tablist.querySelector<HTMLButtonElement>(`#automations-tab-${view}`)?.focus();
  }, [view]);

  function moveSelection(event: KeyboardEvent<HTMLButtonElement>) {
    const index = automationsViews.indexOf(view);
    if (event.key === "ArrowRight") onViewChange(automationsViews[(index + 1) % automationsViews.length]);
    else if (event.key === "ArrowLeft") onViewChange(automationsViews[(index + automationsViews.length - 1) % automationsViews.length]);
    else return;
    setKeyboardSelection(true);
    event.preventDefault();
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2" data-testid="automations">
      <div aria-label={t("automations.tabs")} className="ucd-panel flex shrink-0 items-center gap-1 rounded-lg p-1" ref={tablistRef} role="tablist">
        {tabs.map((tab) => {
          const Icon = tab.icon;
          const selected = tab.view === view;
          return (
            <button
              aria-controls={tab.panelId}
              aria-selected={selected}
              className={cn(
                "ucd-interactive flex h-8 items-center gap-1.5 rounded-md border px-3 text-xs",
                selected ? "border-primary bg-[hsl(var(--nav-active-soft))] font-semibold text-primary" : "border-transparent text-muted-foreground",
              )}
              data-automations-tab={tab.view}
              id={`automations-tab-${tab.view}`}
              key={tab.view}
              onClick={() => { setKeyboardSelection(false); onViewChange(tab.view); }}
              onKeyDown={moveSelection}
              role="tab"
              tabIndex={selected ? 0 : -1}
              type="button"
            >
              <Icon aria-hidden="true" className="h-3.5 w-3.5" />
              {t(tab.labelKey)}
            </button>
          );
        })}
      </div>
      {tabs.map((tab) => {
        const selected = tab.view === view;
        return (
          <div
            aria-labelledby={`automations-tab-${tab.view}`}
            className={cn("min-h-0 min-w-0 flex-1", selected ? "flex" : "hidden")}
            hidden={!selected}
            id={tab.panelId}
            key={tab.view}
            role="tabpanel"
          >
            {visited.has(tab.view) ? (
              tab.view === "loops" ? <LazyFeature className="h-full min-h-0 flex-1" componentProps={{ onInspect: onInspectLoop }} loader={loadLoopCenter} />
                : tab.view === "scheduled" ? <LazyFeature className="h-full min-h-0 flex-1" componentProps={{ active: active && selected, agents, focusOnActivate: !keyboardSelection }} loader={loadScheduledTasks} />
                  : <LazyFeature className="h-full min-h-0 flex-1" componentProps={{}} loader={loadGoalCenter} />
            ) : null}
          </div>
        );
      })}
    </div>
  );
}
