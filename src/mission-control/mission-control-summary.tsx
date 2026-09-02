import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { MissionControlCounts } from "../types/mission-control";
import { MISSION_CONTROL_COUNT_STATES, sameStateSet, type MissionControlFilterState } from "./mission-control-query";

export interface MissionControlSummaryProps {
  counts: MissionControlCounts;
  states: MissionControlFilterState["states"];
  onToggle: (key: keyof MissionControlCounts) => void;
}

/**
 * 16.4: the metric strip is now a row of filter toggles, not decoration -- clicking a count narrows
 * the Run collection to exactly the states `MISSION_CONTROL_COUNT_STATES` maps it to (that mapping
 * is verified exact against both backends' own count projection; see that module's own doc
 * comment), and clicking an already-active count clears it back to "all". `aria-pressed` mirrors
 * whichever filter is currently applied, however it was set -- including from the Toolbar's own
 * status dropdown (16.5), since both read/write the same underlying `states` array.
 *
 * Sized down from the original's `text-lg font-semibold` / `px-3 py-2` card, and no longer given a
 * fixed `min-w-28` ("16.4: reduce large metric-card competition"): once these became interactive
 * controls rather than decoration, they are secondary navigation sitting above the actual Run
 * collection, not the page's own visual focus -- a smaller, content-sized pill row keeps them from
 * out-competing the runs themselves for attention.
 */
export function MissionControlSummary({ counts, states, onToggle }: MissionControlSummaryProps) {
  const { t } = useTranslation();
  const keys = Object.keys(counts) as (keyof MissionControlCounts)[];
  return (
    <div aria-label={t("missionControl.summaryGroup")} className="flex gap-1.5 overflow-x-auto border-b border-border p-2" role="group">
      {keys.map((key) => {
        const pressed = sameStateSet(states, MISSION_CONTROL_COUNT_STATES[key]);
        return (
          <button
            aria-pressed={pressed}
            className={cn(
              "ucd-focus-ring shrink-0 rounded-md border px-2 py-1 text-left transition-colors",
              pressed ? "border-primary bg-primary/10" : "border-border bg-muted/30 hover:bg-muted/50",
            )}
            data-testid={`mission-control-count-${key}`}
            key={key}
            onClick={() => onToggle(key)}
            type="button"
          >
            <p className="text-[10px] text-muted-foreground">{t(`missionControl.count.${key}`)}</p>
            <p className={cn("text-sm font-semibold tabular-nums", pressed && "text-primary")}>{counts[key]}</p>
          </button>
        );
      })}
    </div>
  );
}
