import { Search } from "lucide-react";
import type { RefObject } from "react";
import { useTranslation } from "react-i18next";
import type { AgentRegistryEntry } from "../types/agent";
import { FilterPopover, type FilterField } from "../ui/filter-popover/FilterPopover";
import { Toolbar } from "../ui/toolbar/Toolbar";
import { frequencyKinds } from "./scheduled-task-presentation";
import {
  isScheduledTaskFilterActive, nextRunRangeBuckets, scheduledTaskLatestStatuses,
  type ScheduledTaskFilterState,
} from "./scheduled-task-query";

const fieldClass = "h-8 rounded-md border border-input bg-background px-2 text-xs";

export interface ScheduledTaskFiltersProps {
  agents: AgentRegistryEntry[];
  filter: ScheduledTaskFilterState;
  onFilterChange: (patch: Partial<ScheduledTaskFilterState>) => void;
  onClearFilters: () => void;
  searchInputRef: RefObject<HTMLInputElement | null>;
}

/**
 * 19.4: `Toolbar` + `FilterPopover` composition, the same bundled trigger+chips shape
 * `MissionControlToolbar`/`WorkBoardToolbar` already established (16.5/14.1). Extracted into its
 * own file purely to keep `scheduled-tasks-panel.tsx` well under its 300-line budget, not because
 * this list has its own saved views or sort/view controls -- it has neither, since 19.4/19.5 asked
 * for neither.
 *
 * Agent lists the full live registry (not derived from which agents the currently-loaded tasks
 * happen to use), mirroring `MissionControlToolbar`'s own Agent field precedent exactly -- Agents
 * are a genuine fixed registry here, unlike Work Board's per-item project paths (which have no
 * fixed registry to draw from, per `work-board-query.ts`'s own doc comment on why Work Board has no
 * Agent field at all). `FilterPopover` only ever renders `<select>` fields, so this is a dropdown
 * of real display names, not a free-text field.
 *
 * "Recurrence" (19.4/19.5's own task wording) is deliberately labeled and keyed as "frequency" --
 * this codebase's own established vocabulary everywhere else a task's schedule kind is shown
 * (`ScheduledTaskFrequency`, `scheduledTasks.frequency.*`, `formatScheduledTaskFrequency`) --
 * matching 19.15's own "keep the established term, don't introduce a second one" precedent.
 */
export function ScheduledTaskFilters({ agents, filter, onClearFilters, onFilterChange, searchInputRef }: ScheduledTaskFiltersProps) {
  const { t } = useTranslation();
  const sortedAgents = [...agents].sort((left, right) => left.displayName.localeCompare(right.displayName));

  const fields: FilterField[] = [
    {
      id: "agent", label: t("scheduledTasks.filterAgent"), value: filter.agentId, defaultValue: "",
      onChange: (value) => onFilterChange({ agentId: value }),
      options: [
        { value: "", label: t("scheduledTasks.allAgents") },
        ...sortedAgents.map((agent) => ({ value: agent.id, label: agent.displayName })),
      ],
    },
    {
      id: "frequency", label: t("scheduledTasks.filterFrequency"), value: filter.frequencyKind, defaultValue: "",
      onChange: (value) => onFilterChange({ frequencyKind: value as ScheduledTaskFilterState["frequencyKind"] }),
      options: [
        { value: "", label: t("scheduledTasks.allFrequencies") },
        ...frequencyKinds.map((kind) => ({ value: kind, label: t(`scheduledTasks.frequency.${kind}`) })),
      ],
    },
    {
      id: "enabled", label: t("scheduledTasks.filterEnabled"), value: filter.enabled, defaultValue: "",
      onChange: (value) => onFilterChange({ enabled: value as ScheduledTaskFilterState["enabled"] }),
      options: [
        { value: "", label: t("scheduledTasks.allEnabledStates") },
        { value: "true", label: t("scheduledTasks.enabled") },
        { value: "false", label: t("scheduledTasks.disabled") },
      ],
    },
    {
      id: "status", label: t("scheduledTasks.filterStatus"), value: filter.status, defaultValue: "",
      onChange: (value) => onFilterChange({ status: value as ScheduledTaskFilterState["status"] }),
      options: [
        { value: "", label: t("scheduledTasks.allStatuses") },
        ...scheduledTaskLatestStatuses.map((status) => ({ value: status, label: t(`scheduledTasks.status.${status}`) })),
      ],
    },
    {
      id: "nextRunRange", label: t("scheduledTasks.filterNextRun"), value: filter.nextRunRange, defaultValue: "all",
      onChange: (value) => onFilterChange({ nextRunRange: value as ScheduledTaskFilterState["nextRunRange"] }),
      options: nextRunRangeBuckets.map((bucket) => ({ value: bucket, label: t(`scheduledTasks.nextRunRange.${bucket}`) })),
    },
  ];

  const filtersActive = isScheduledTaskFilterActive(filter);

  return (
    <Toolbar
      activeFilters={
        <div className="flex flex-wrap items-center gap-1.5">
          <FilterPopover fields={fields} triggerLabel={t("scheduledTasks.filters")} />
          {filtersActive ? (
            <button className="ucd-focus-ring rounded-md border border-input px-2 py-1 text-xs hover:bg-accent" onClick={onClearFilters} type="button">
              {t("scheduledTasks.clearFilters")}
            </button>
          ) : null}
        </div>
      }
      search={
        <label className="relative block min-w-40 flex-1">
          <Search aria-hidden="true" className="absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
          <span className="sr-only">{t("scheduledTasks.search")}</span>
          <input
            className={`${fieldClass} w-full pl-9`}
            onChange={(event) => onFilterChange({ search: event.target.value })}
            placeholder={t("scheduledTasks.search")}
            ref={searchInputRef}
            value={filter.search}
          />
        </label>
      }
      searchInputRef={searchInputRef}
    />
  );
}
