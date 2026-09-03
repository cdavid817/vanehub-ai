import { Archive, CheckSquare, FilterX, Inbox, Search, X } from "lucide-react";
import { type RefObject } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { WorkItemPriority, WorkItemSourceKind, WorkItemStage } from "../types/work-board";
import { workItemPriorities, workItemSourceKinds, workItemStages } from "../types/work-board";
import { FilterPopover, type FilterField } from "../ui/filter-popover/FilterPopover";
import { Toolbar } from "../ui/toolbar/Toolbar";
import { workItemDueBuckets, type WorkItemDueBucket } from "./work-board-filter";
import { fieldClass } from "./work-board-form";
import {
  ALL_PROJECTS, workBoardGroupings, workBoardSorts,
  type WorkBoardGrouping, type WorkBoardQuery, type WorkBoardSort,
} from "./work-board-query";
import { WorkBoardSavedViewMenu } from "./work-board-saved-view-menu";
import type { WorkBoardSavedView } from "./work-board-saved-views";
import { WorkBoardWipLimitMenu } from "./work-board-wip-limit-menu";
import type { WorkBoardWipLimits } from "./work-board-wip-limits";

export interface WorkBoardToolbarProps {
  archived: boolean;
  onToggleArchived: () => void;
  query: WorkBoardQuery;
  onQueryChange: (patch: Partial<WorkBoardQuery>) => void;
  filtersActive: boolean;
  onClearFilters: () => void;
  projects: { label: string; value: string }[];
  searchInputRef: RefObject<HTMLInputElement | null>;
  savedViews: WorkBoardSavedView[];
  onApplySavedView: (view: WorkBoardSavedView) => void;
  onDeleteSavedView: (id: string) => void;
  onSaveCurrentView: (name: string) => void;
  /** 14.12: whether select/batch mode is currently active. */
  batchMode: boolean;
  onToggleBatchMode: () => void;
  /** 14.14: presentation-only soft limits, read and edited entirely client-side. */
  wipLimits: WorkBoardWipLimits;
  onSaveWipLimits: (limits: WorkBoardWipLimits) => void;
}

/**
 * 14.1: design.md Decision 11's own "统一 Toolbar: search; filter trigger; active filters; saved
 * view; view/sort; batch mode" shape, built on `src/ui/toolbar/Toolbar.tsx` (task 3.4's shared
 * primitive) for the row layout. The filter trigger and its active-filter chips are one bundled
 * unit here (`FilterPopover`, already production-proven in session-sidebar.tsx), not `Toolbar`'s
 * own separate `onFilterTrigger`+`activeFilters` slots: those two props assume the caller renders
 * chips independently of the trigger, which is exactly what `FilterPopover` does not do (it
 * renders both together), and `FilterBar` (the other candidate for `activeFilters`) would then
 * double-render a chip per active field alongside FilterPopover's own. Composing `Toolbar` with a
 * bundled trigger+chips widget through its `activeFilters` slot instead avoids that duplication
 * without forking either shared primitive. Clear-all is a plain adjacent button (not FilterBar's
 * built-in one, for the same reason), preserving the pre-existing clearFilters affordance.
 */
export function WorkBoardToolbar({
  archived, onToggleArchived, query, onQueryChange, filtersActive, onClearFilters,
  projects, searchInputRef, savedViews, onApplySavedView, onDeleteSavedView, onSaveCurrentView,
  batchMode, onToggleBatchMode, wipLimits, onSaveWipLimits,
}: WorkBoardToolbarProps) {
  const { t } = useTranslation();

  const fields: FilterField[] = [
    {
      id: "source", label: t("todoBoard.sourceFilter"), value: query.source, defaultValue: "all",
      onChange: (value) => onQueryChange({ source: value as WorkItemSourceKind | "all" }),
      options: [{ value: "all", label: t("todoBoard.source.all") }, ...workItemSourceKinds.map((kind) => ({ value: kind, label: t(`todoBoard.source.${kind}`) }))],
    },
    {
      id: "priority", label: t("todoBoard.priorityFilter"), value: query.priority, defaultValue: "all",
      onChange: (value) => onQueryChange({ priority: value as WorkItemPriority | "all" }),
      options: [{ value: "all", label: t("todoBoard.priority.all") }, ...workItemPriorities.map((kind) => ({ value: kind, label: t(`todoBoard.priority.${kind}`) }))],
    },
    {
      id: "stage", label: t("todoBoard.stageFilter"), value: query.stage, defaultValue: "all",
      onChange: (value) => onQueryChange({ stage: value as WorkItemStage | "all" }),
      options: [{ value: "all", label: t("todoBoard.stage.all") }, ...workItemStages.map((stage) => ({ value: stage, label: t(`todoBoard.stage.${stage}`) }))],
    },
    {
      id: "due", label: t("todoBoard.dueFilter"), value: query.due, defaultValue: "all",
      onChange: (value) => onQueryChange({ due: value as WorkItemDueBucket }),
      options: workItemDueBuckets.map((bucket) => ({ value: bucket, label: t(`todoBoard.due.${bucket}`) })),
    },
    {
      id: "project", label: t("todoBoard.projectFilter"), value: query.project, defaultValue: ALL_PROJECTS,
      onChange: (value) => onQueryChange({ project: value }),
      options: [{ value: ALL_PROJECTS, label: t("todoBoard.project.all") }, ...projects],
    },
  ];

  return (
    <Toolbar
      batchModeSlot={
        <Button
          aria-pressed={batchMode}
          className="h-8 px-2.5 text-xs"
          onClick={onToggleBatchMode}
          size="sm"
          type="button"
          variant={batchMode ? "default" : "outline"}
        >
          {batchMode ? <X aria-hidden="true" className="h-3.5 w-3.5" /> : <CheckSquare aria-hidden="true" className="h-3.5 w-3.5" />}
          {batchMode ? t("todoBoard.batch.exit") : t("todoBoard.batch.trigger")}
        </Button>
      }
      activeFilters={
        <div className="flex flex-wrap items-center gap-1.5">
          <FilterPopover fields={fields} triggerLabel={t("todoBoard.filters")} />
          {filtersActive ? (
            <Button className="h-7 px-2 text-xs" onClick={onClearFilters} size="sm" type="button" variant="ghost">
              <FilterX aria-hidden="true" className="h-3 w-3" />{t("todoBoard.clearFilters")}
            </Button>
          ) : null}
        </div>
      }
      search={
        <label className="relative block min-w-48 flex-1">
          <Search aria-hidden="true" className="absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
          <span className="sr-only">{t("todoBoard.search")}</span>
          <input
            className={`${fieldClass} w-full pl-9`}
            onChange={(event) => onQueryChange({ text: event.target.value })}
            placeholder={t("todoBoard.search")}
            ref={searchInputRef}
            value={query.text}
          />
        </label>
      }
      searchInputRef={searchInputRef}
      sortControl={
        <select aria-label={t("todoBoard.sort")} className={fieldClass} onChange={(event) => onQueryChange({ sort: event.target.value as WorkBoardSort })} value={query.sort}>
          {workBoardSorts.map((sort) => <option key={sort} value={sort}>{t(`todoBoard.sort.${sort}`)}</option>)}
        </select>
      }
      viewControl={
        <div className="flex items-center gap-1.5">
          <div className="ucd-segmented grid grid-cols-2 gap-1 rounded-md p-1">
            <button aria-pressed={query.presentation === "board"} className={`h-7 rounded px-2 text-xs ${query.presentation === "board" ? "bg-background font-semibold text-primary" : "text-muted-foreground hover:bg-muted"}`} onClick={() => onQueryChange({ presentation: "board" })} type="button">
              {t("todoBoard.presentation.board")}
            </button>
            <button aria-pressed={query.presentation === "list"} className={`h-7 rounded px-2 text-xs ${query.presentation === "list" ? "bg-background font-semibold text-primary" : "text-muted-foreground hover:bg-muted"}`} onClick={() => onQueryChange({ presentation: "list" })} type="button">
              {t("todoBoard.presentation.list")}
            </button>
          </div>
          {query.presentation === "list" ? (
            <select aria-label={t("todoBoard.grouping")} className={fieldClass} onChange={(event) => onQueryChange({ grouping: event.target.value as WorkBoardGrouping })} value={query.grouping}>
              {workBoardGroupings.map((grouping) => <option key={grouping} value={grouping}>{t(`todoBoard.grouping.${grouping}`)}</option>)}
            </select>
          ) : null}
          <Button onClick={onToggleArchived} size="sm" type="button" variant="outline">
            {archived ? <Inbox aria-hidden="true" /> : <Archive aria-hidden="true" />}
            {archived ? t("todoBoard.active") : t("todoBoard.archived")}
          </Button>
          <WorkBoardSavedViewMenu onApply={onApplySavedView} onDelete={onDeleteSavedView} onSave={onSaveCurrentView} savedViews={savedViews} />
          <WorkBoardWipLimitMenu limits={wipLimits} onSave={onSaveWipLimits} />
        </div>
      }
    />
  );
}
