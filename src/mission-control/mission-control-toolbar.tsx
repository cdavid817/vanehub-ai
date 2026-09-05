import { RefreshCw } from "lucide-react";
import { type RefObject } from "react";
import { useTranslation } from "react-i18next";
import type { AgentRunState } from "../types/agent-run";
import type { AgentRegistryEntry } from "../types/agent";
import type { MissionControlSort } from "../types/mission-control";
import { FilterPopover, type FilterField } from "../ui/filter-popover/FilterPopover";
import { Toolbar } from "../ui/toolbar/Toolbar";
import { isMissionControlFilterActive, MISSION_CONTROL_STATUS_OPTIONS, type MissionControlFilterState } from "./mission-control-query";
import { MissionControlSavedViewMenu } from "./mission-control-saved-view-menu";
import type { MissionControlSavedView } from "./mission-control-saved-views";

const fieldClass = "h-8 rounded-md border border-input bg-background px-2 text-xs";

export interface MissionControlToolbarProps {
  filter: MissionControlFilterState;
  onFilterChange: (patch: Partial<MissionControlFilterState>) => void;
  onClearFilters: () => void;
  agents: AgentRegistryEntry[];
  loading: boolean;
  onRefresh: () => void;
  searchInputRef: RefObject<HTMLInputElement | null>;
  savedViews: MissionControlSavedView[];
  onApplySavedView: (view: MissionControlSavedView) => void;
  onDeleteSavedView: (id: string) => void;
  onSaveCurrentView: (name: string) => void;
}

/**
 * 16.5: migrates the header's own plain, always-visible agentId/projectId/status/runner/sort
 * controls into `Toolbar` + `FilterPopover`, the same bundled trigger+chips composition
 * `WorkBoardToolbar` established (14.1) -- see that file's own doc comment for why FilterPopover
 * fills `Toolbar`'s `activeFilters` slot directly rather than `Toolbar`'s own separate
 * `onFilterTrigger`/`activeFilters` props.
 *
 * Agent becomes a dropdown of real registry display names, not a migrated free-text input, for two
 * independent reasons that both point the same way: `FilterPopover` only ever renders `<select>`
 * fields (it has no free-text field shape at all), and `getMissionControlOverview`'s own `agentId`
 * filter is an exact-match query on both backends (`owner_id = ?` in
 * mission_control_repository.rs; `run.agentId === query.agentId` in web-mission-control-client.ts)
 * -- free text could never partially match anyway, so a dropdown of real names is a strict UX
 * improvement, not just a style choice.
 *
 * Project id keeps its pre-existing free-text shape -- there is nothing to build a dropdown of (its
 * value is unconditionally null on every real run today, an already-flagged backend gap, not
 * something to build display logic around) -- and moves into `Toolbar`'s own `search` slot, the
 * same slot `WorkBoardToolbar` puts its one free-text field into (`FilterPopover` cannot host it
 * either, for the same select-only reason as above).
 *
 * "Attention" (16.5's own task title) is not a separate control here: `MissionControlQuery` has no
 * client-settable attention-only field (the Rust side's `attention_only` is `#[serde(skip)]`,
 * internal-only, used to scope the Attention *section* of the overview, not exposed as a filter) --
 * inventing one would be exactly the fabricated-field trap this task's own "don't guess" warnings
 * elsewhere are about. Ordering is covered by `sort`, which already has an `"attention"` value.
 */
export function MissionControlToolbar({
  filter, onFilterChange, onClearFilters, agents, loading, onRefresh, searchInputRef,
  savedViews, onApplySavedView, onDeleteSavedView, onSaveCurrentView,
}: MissionControlToolbarProps) {
  const { t } = useTranslation();
  const sortedAgents = [...agents].sort((left, right) => left.displayName.localeCompare(right.displayName));

  const fields: FilterField[] = [
    {
      id: "agent", label: t("missionControl.filterAgent"), value: filter.agentId, defaultValue: "",
      onChange: (value) => onFilterChange({ agentId: value }),
      options: [
        { value: "", label: t("missionControl.allAgents") },
        ...sortedAgents.map((agent) => ({ value: agent.id, label: agent.displayName })),
      ],
    },
    {
      id: "status", label: t("missionControl.filterStatus"), value: filter.states.length === 1 ? filter.states[0] : "", defaultValue: "",
      onChange: (value) => onFilterChange({ states: value ? [value as AgentRunState] : [] }),
      options: [
        { value: "", label: t("missionControl.allStatuses") },
        ...MISSION_CONTROL_STATUS_OPTIONS.map((state) => ({ value: state, label: t(`missionControl.state.${state}`) })),
      ],
    },
    {
      id: "runner", label: t("missionControl.filterRunner"), value: filter.runner, defaultValue: "",
      onChange: (value) => onFilterChange({ runner: value as MissionControlFilterState["runner"] }),
      options: [
        { value: "", label: t("missionControl.allRunners") },
        { value: "local", label: t("runner.kind.local") },
        { value: "ssh", label: t("runner.kind.ssh") },
      ],
    },
  ];

  const filtersActive = isMissionControlFilterActive(filter);

  return (
    <Toolbar
      activeFilters={
        <div className="flex flex-wrap items-center gap-1.5">
          <FilterPopover fields={fields} triggerLabel={t("missionControl.filters")} />
          {filtersActive ? (
            <button className="ucd-focus-ring rounded-md border border-input px-2 py-1 text-xs hover:bg-accent" onClick={onClearFilters} type="button">
              {t("missionControl.clearFilters")}
            </button>
          ) : null}
        </div>
      }
      search={
        <label className="relative block min-w-40 flex-1">
          <span className="sr-only">{t("missionControl.filterProject")}</span>
          <input
            className={`${fieldClass} w-full`}
            onChange={(event) => onFilterChange({ projectId: event.target.value })}
            placeholder={t("missionControl.filterProject")}
            ref={searchInputRef}
            value={filter.projectId}
          />
        </label>
      }
      searchInputRef={searchInputRef}
      sortControl={
        <select aria-label={t("missionControl.sort")} className={fieldClass} onChange={(event) => onFilterChange({ sort: event.target.value as MissionControlSort })} value={filter.sort}>
          <option value="attention">{t("missionControl.sortAttention")}</option>
          <option value="newest">{t("missionControl.sortNewest")}</option>
          <option value="oldest">{t("missionControl.sortOldest")}</option>
        </select>
      }
      viewControl={
        <div className="flex flex-wrap items-center gap-1.5">
          <MissionControlSavedViewMenu onApply={onApplySavedView} onDelete={onDeleteSavedView} onSave={onSaveCurrentView} savedViews={savedViews} />
          <button aria-label={t("missionControl.refresh")} className="ucd-interactive grid h-8 w-8 place-items-center rounded-md border border-input" onClick={onRefresh} title={t("missionControl.refresh")} type="button">
            <RefreshCw aria-hidden="true" className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          </button>
        </div>
      }
    />
  );
}
