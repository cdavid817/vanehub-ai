import { useTranslation } from "react-i18next";
import { LazyFeature, type LazyFeatureLoader } from "../components/lazy-feature";
import { cn } from "../lib/utils";
import type { PlanSection } from "./workbench-route";

const TABS: { section: PlanSection["section"]; labelKey: string }[] = [
  { section: "board", labelKey: "layout.activityBar.todoBoard" },
  { section: "goals", labelKey: "layout.activityBar.goals" },
];

const loadWorkBoard: LazyFeatureLoader<Record<string, never>> = () => import("../work-board/work-board")
  .then((module) => ({ default: module.WorkBoard }));
type GoalCenterProps = { goalId?: string; onSelectGoal?: (goalId: string | undefined) => void };
const loadGoalCenter: LazyFeatureLoader<GoalCenterProps> = () => import("../goal-center/goal-center")
  .then((module) => ({ default: module.GoalCenter }));

export interface PlanDestinationProps {
  location: PlanSection;
  onSectionChange: (section: PlanSection) => void;
}

/**
 * `WorkBoard` is a zero-prop, fully self-contained component today (confirmed by reading it
 * directly) — `viewId`/`workItemId` on `PlanSection` are not consumed here. That is a real gap,
 * not an oversight: it has no injectable initial-selection prop, so making the URL drive its
 * selected item is content work for a later milestone, the same reasoning as `RunsDestination`'s
 * own `definitionId`/`loopRunId` gap for LoopCenter.
 *
 * 15.1: `goalId` is no longer in that boat — `GoalCenter` now takes it as its current/initial
 * selection and reports selection changes back through `onSelectGoal`, wired below to
 * `onSectionChange` the same way the tab buttons already are and the same way `scheduleId`/
 * `onSelectSchedule` are wired in `RunsDestination` (19.3), so Back/forward and reload restore
 * the same selected goal.
 */
export function PlanDestination({ location, onSectionChange }: PlanDestinationProps) {
  const { t } = useTranslation();

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
      <div className="min-h-0 flex-1 p-2">
        {/* Two separate, permanently loader-fixed `LazyFeature` call sites -- mirrors
            runs-destination.tsx's own Loops/Schedules split -- rather than one ternary sharing a
            single slot between `loadWorkBoard` and `loadGoalCenter`. `LazyFeature` picks its lazy
            component once via `useState(() => lazy(loader))` (a lazy initializer, which React only
            evaluates on that instance's first mount): a single ternary slot re-renders the *same*
            `LazyFeature` instance with a new `loader` prop on every section switch, which that
            initializer silently ignores, so the previously loaded feature stayed on screen forever
            regardless of which tab was active. Each section its own call site instead means
            switching sections mounts a genuinely new `LazyFeature` (and a fresh `lazy()` load) every
            time, the same fix `RunsDestination`'s own doc comment already explains for that file. */}
        {location.section === "board" ? (
          <LazyFeature className="h-full min-h-0" componentProps={{}} loader={loadWorkBoard} />
        ) : null}
        {location.section === "goals" ? (
          <LazyFeature
            className="h-full min-h-0"
            componentProps={{
              goalId: location.goalId,
              onSelectGoal: (nextGoalId: string | undefined) => onSectionChange({ section: "goals", goalId: nextGoalId }),
            }}
            loader={loadGoalCenter}
          />
        ) : null}
      </div>
    </div>
  );
}
