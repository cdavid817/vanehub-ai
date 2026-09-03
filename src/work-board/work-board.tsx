import { Loader2, Plus } from "lucide-react";
import { useMemo, useRef, useState, type DragEvent } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { useMediaQuery } from "../hooks/use-media-query";
import { normalizeDisplayPath } from "../lib/session-path";
import { PageHeader } from "../ui/page-header/PageHeader";
import { Sheet } from "../ui/sheet/Sheet";
import type { WorkItem, WorkItemStage } from "../types/work-board";
import { workItemStages } from "../types/work-board";
import { CREATE_MUTATION_KEY, useWorkBoardActions } from "./use-work-board-actions";
import { useWorkBoardBatch } from "./use-work-board-batch";
import { WorkBoardBatchPanel } from "./work-board-batch-panel";
import { filterWorkItems, matchesDueBucket } from "./work-board-filter";
import { WorkItemForm } from "./work-board-form";
import { WorkBoardColumn } from "./work-board-column";
import { WorkBoardList } from "./work-board-list";
import {
  ALL_PROJECTS, defaultWorkBoardQuery, isWorkBoardFilterActive, sortWorkItems, toWorkBoardFilters,
  type WorkBoardQuery,
} from "./work-board-query";
import {
  applyWorkBoardSavedView, captureWorkBoardSavedView, readWorkBoardSavedViews, writeWorkBoardSavedViews,
  type WorkBoardSavedView,
} from "./work-board-saved-views";
import { WorkBoardToolbar } from "./work-board-toolbar";
import { readWorkBoardWipLimits, writeWorkBoardWipLimits, type WorkBoardWipLimits } from "./work-board-wip-limits";

/**
 * 14.1: the Page Header's own bounded summary slot (design.md Decision 11), matching
 * GoalCenterSummary's own real-not-fabricated-metric precedent (goal-center.tsx) -- a total count
 * over the current archived/active scope plus, when non-zero, how many of those are overdue,
 * reusing `matchesDueBucket` (the same predicate the due filter itself applies) rather than a
 * second ad hoc definition of "overdue".
 */
function WorkBoardSummary({ items }: { items: WorkItem[] }) {
  const { t } = useTranslation();
  const overdueCount = items.filter((item) => matchesDueBucket(item, "overdue")).length;
  return (
    <span className="flex flex-wrap items-center gap-2 text-sm font-normal text-muted-foreground">
      <span>{t("todoBoard.summary.total", { count: items.length })}</span>
      {overdueCount > 0 ? <Badge tone="danger">{t("todoBoard.summary.overdue", { count: overdueCount })}</Badge> : null}
    </span>
  );
}

/** Text for every non-default dimension, in the same order the filter chips render -- used only
 *  to explain a filtered-empty column/list ("why", not just "that"), never to drive filtering. */
function describeActiveFilters(query: WorkBoardQuery, projectLabel: (path: string) => string, t: (key: string, options?: Record<string, unknown>) => string): string {
  const parts: string[] = [];
  if (query.text.trim()) parts.push(`${t("todoBoard.search")}: ${query.text.trim()}`);
  if (query.source !== "all") parts.push(`${t("todoBoard.sourceFilter")}: ${t(`todoBoard.source.${query.source}`)}`);
  if (query.priority !== "all") parts.push(`${t("todoBoard.priorityFilter")}: ${t(`todoBoard.priority.${query.priority}`)}`);
  if (query.stage !== "all") parts.push(`${t("todoBoard.stageFilter")}: ${t(`todoBoard.stage.${query.stage}`)}`);
  if (query.due !== "all") parts.push(`${t("todoBoard.dueFilter")}: ${t(`todoBoard.due.${query.due}`)}`);
  if (query.project !== ALL_PROJECTS) parts.push(`${t("todoBoard.projectFilter")}: ${projectLabel(query.project)}`);
  return parts.join(", ");
}

export function WorkBoard() {
  const { t } = useTranslation();
  const compact = useMediaQuery("(max-width: 900px)");
  const [query, setQuery] = useState<WorkBoardQuery>(defaultWorkBoardQuery);
  const [archived, setArchived] = useState(false);
  const [creating, setCreating] = useState(false);
  const [editing, setEditing] = useState<WorkItem | null>(null);
  const [savedViews, setSavedViews] = useState<WorkBoardSavedView[]>(() => readWorkBoardSavedViews());
  const [wipLimits, setWipLimits] = useState<WorkBoardWipLimits>(() => readWorkBoardWipLimits());
  const searchInputRef = useRef<HTMLInputElement>(null);

  const { archive, create, error, items, loading, move, mutations, remove, restore, update } = useWorkBoardActions(archived);
  const updateQuery = (patch: Partial<WorkBoardQuery>) => setQuery((current) => ({ ...current, ...patch }));

  // The option value stays the stored path so filtering keeps matching; only the label is
  // normalized, because a stored `\\?\` prefix is a Windows API detail, not something to read.
  const projects = useMemo(
    () => [...new Set(items.flatMap((item) => item.projectPath ? [item.projectPath] : []))]
      .sort()
      .map((path) => ({ label: normalizeDisplayPath(path), value: path })),
    [items],
  );
  const projectLabel = (path: string) => (path === ALL_PROJECTS ? t("todoBoard.project.all") : normalizeDisplayPath(path));

  const visible = useMemo(() => filterWorkItems(items, toWorkBoardFilters(query, archived)), [archived, items, query]);
  const sorted = useMemo(() => sortWorkItems(visible, query.sort), [visible, query.sort]);

  const drop = (event: DragEvent<HTMLElement>, stage: WorkItemStage) => {
    event.preventDefault();
    const item = items.find((candidate) => candidate.id === event.dataTransfer.getData("text/work-item"));
    if (item) void move(item, stage);
  };

  const filtersActive = isWorkBoardFilterActive(query);
  const filterSummary = filtersActive ? describeActiveFilters(query, projectLabel, t) : undefined;
  const clearFilters = () => updateQuery({ text: "", project: ALL_PROJECTS, source: "all", priority: "all", due: "all", stage: "all" });

  // 14.12: eligible items are whatever the reader can currently see selecting -- `sorted`, not the
  // unfiltered `items` -- so a filter change prunes stale selections the same way
  // session-sidebar.tsx's own batch mode already does.
  const batch = useWorkBoardBatch({ archive, move, visibleItems: sorted });
  function saveWipLimits(next: WorkBoardWipLimits) {
    setWipLimits(next);
    writeWorkBoardWipLimits(next);
  }

  function applySavedView(view: WorkBoardSavedView) {
    setQuery(applyWorkBoardSavedView(view));
  }
  function saveCurrentView(name: string) {
    const next = [...savedViews, captureWorkBoardSavedView(query, name, crypto.randomUUID())];
    setSavedViews(next);
    writeWorkBoardSavedViews(next);
  }
  function deleteSavedView(id: string) {
    const next = savedViews.filter((view) => view.id !== id);
    setSavedViews(next);
    writeWorkBoardSavedViews(next);
  }

  return <section aria-labelledby="todo-board-title" className="ucd-panel flex h-full min-h-0 flex-1 flex-col overflow-hidden rounded-lg" id="todo-board">
    <PageHeader
      className="shrink-0 p-3 md:p-4"
      description={t("todoBoard.subtitle")}
      primaryAction={
        <Button onClick={() => { setEditing(null); setCreating(true); }} size="sm" type="button">
          <Plus aria-hidden="true" />{t("todoBoard.new")}
        </Button>
      }
      statusSummary={!(loading && !items.length) ? <WorkBoardSummary items={items} /> : null}
      title={t("todoBoard.title")}
    />
    <div className="shrink-0 border-b border-border p-3 md:p-4">
      <WorkBoardToolbar
        archived={archived}
        batchMode={batch.batchMode}
        filtersActive={filtersActive}
        onApplySavedView={applySavedView}
        onClearFilters={clearFilters}
        onDeleteSavedView={deleteSavedView}
        onQueryChange={updateQuery}
        onSaveCurrentView={saveCurrentView}
        onSaveWipLimits={saveWipLimits}
        onToggleArchived={() => setArchived((value) => !value)}
        onToggleBatchMode={() => (batch.batchMode ? batch.exit() : batch.enter())}
        projects={projects}
        query={query}
        savedViews={savedViews}
        searchInputRef={searchInputRef}
        wipLimits={wipLimits}
      />
      {filtersActive ? <p className="mt-2 text-[11px] text-muted-foreground" role="status">{t("todoBoard.filtersActive", { count: visible.length })}</p> : null}
    </div>

    {batch.batchMode ? (
      <WorkBoardBatchPanel
        items={sorted}
        onArchive={() => void batch.run({ kind: "archive" })}
        onClearSelection={batch.clearSelection}
        onExit={batch.exit}
        onMove={(stage) => void batch.run({ kind: "move", stage })}
        onSelectAllVisible={batch.selectAllVisible}
        outcome={batch.outcome}
        running={batch.running}
        selectedIds={batch.selectedIds}
      />
    ) : null}

    {creating
      ? <Sheet closeDisabled={mutations.get(CREATE_MUTATION_KEY)?.pending} onClose={() => setCreating(false)} placement="right" title={t("todoBoard.new")}>
          <WorkItemForm mutation={mutations.get(CREATE_MUTATION_KEY)} onCancel={() => setCreating(false)} onSubmit={(input) => void create(input, () => setCreating(false))} submitLabel={t("todoBoard.create")} />
        </Sheet>
      : null}
    {editing
      ? <Sheet closeDisabled={mutations.get(editing.id)?.pending} onClose={() => setEditing(null)} placement="right" title={t("todoBoard.edit")}>
          <WorkItemForm item={editing} mutation={mutations.get(editing.id)} onCancel={() => setEditing(null)} onSubmit={(input) => void update(editing, input, () => setEditing(null))} submitLabel={t("todoBoard.save")} />
        </Sheet>
      : null}

    {error ? <p className="m-3 rounded border border-destructive/50 bg-destructive/10 p-2 text-sm text-destructive" role="alert">{error}</p> : null}
    {loading && !items.length
      ? <div className="grid flex-1 place-items-center"><Loader2 aria-label={t("todoBoard.loading")} className="animate-spin" /></div>
      // 14.13: at a compact width, the grouped Stage List (vertical, no drag target) replaces the
      // side-by-side Kanban columns entirely -- forcing `grouping="stage"` regardless of
      // `query.presentation`/`query.grouping` matches this branch's own pre-existing precedent of
      // overriding the wide layout's presentation choice at compact widths.
      : compact
        ? <WorkBoardList
            batchMode={batch.batchMode}
            filterSummary={filterSummary}
            filtersActive={filtersActive}
            grouping="stage"
            items={sorted}
            mutations={mutations.registry}
            onArchive={(item) => void archive(item)}
            onDelete={(item) => void remove(item)}
            onDismissError={(item) => mutations.clear(item.id)}
            onEdit={(item) => { setCreating(false); setEditing(item); }}
            onMove={(item, target) => void move(item, target)}
            onRestore={(item) => void restore(item)}
            onToggleSelected={(item) => batch.toggle(item.id)}
            selectedIds={batch.selectedIds}
            wipLimits={wipLimits}
          />
        : query.presentation === "board"
          ? <div className="flex min-h-0 flex-1 gap-3 overflow-x-auto p-3">
              {workItemStages.map((stage) => (
                <WorkBoardColumn
                  batchMode={batch.batchMode}
                  filterSummary={filterSummary}
                  filtersActive={filtersActive}
                  items={sorted.filter((item) => item.stage === stage)}
                  key={stage}
                  mutations={mutations.registry}
                  onArchive={(item) => void archive(item)}
                  onDelete={(item) => void remove(item)}
                  onDismissError={(item) => mutations.clear(item.id)}
                  onDrop={drop}
                  onEdit={(item) => { setCreating(false); setEditing(item); }}
                  onMove={(item, target) => void move(item, target)}
                  onRestore={(item) => void restore(item)}
                  onToggleSelected={(item) => batch.toggle(item.id)}
                  selectedIds={batch.selectedIds}
                  stage={stage}
                  wipLimit={wipLimits[stage]}
                />
              ))}
            </div>
          : <WorkBoardList
              batchMode={batch.batchMode}
              filterSummary={filterSummary}
              filtersActive={filtersActive}
              grouping={query.grouping}
              items={sorted}
              mutations={mutations.registry}
              onArchive={(item) => void archive(item)}
              onDelete={(item) => void remove(item)}
              onDismissError={(item) => mutations.clear(item.id)}
              onEdit={(item) => { setCreating(false); setEditing(item); }}
              onMove={(item, target) => void move(item, target)}
              onRestore={(item) => void restore(item)}
              onToggleSelected={(item) => batch.toggle(item.id)}
              selectedIds={batch.selectedIds}
              wipLimits={wipLimits}
            />}
  </section>;
}
