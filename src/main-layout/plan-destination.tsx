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
const loadGoalCenter: LazyFeatureLoader<Record<string, never>> = () => import("../goal-center/goal-center")
  .then((module) => ({ default: module.GoalCenter }));

export interface PlanDestinationProps {
  location: PlanSection;
  onSectionChange: (section: PlanSection) => void;
}

/**
 * `WorkBoard`/`GoalCenter` are zero-prop, fully self-contained components today (confirmed by
 * reading both directly) — `viewId`/`workItemId`/`goalId` on `PlanSection` are not consumed here.
 * That is a real gap, not an oversight: neither component has an injectable initial-selection
 * prop, so making the URL drive the selected item is content work for a later milestone.
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
        {location.section === "board" ? (
          <LazyFeature className="h-full min-h-0" componentProps={{}} loader={loadWorkBoard} />
        ) : (
          <LazyFeature className="h-full min-h-0" componentProps={{}} loader={loadGoalCenter} />
        )}
      </div>
    </div>
  );
}
