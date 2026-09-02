import { Archive, FilterX, Inbox, Loader2, Plus, Search } from "lucide-react";
import { useMemo, useState, type DragEvent } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { useMediaQuery } from "../hooks/use-media-query";
import { normalizeDisplayPath } from "../lib/session-path";
import type { WorkItem, WorkItemPriority, WorkItemSourceKind, WorkItemStage } from "../types/work-board";
import { workItemPriorities, workItemSourceKinds, workItemStages } from "../types/work-board";
import { CREATE_MUTATION_KEY, useWorkBoardActions } from "./use-work-board-actions";
import { WorkBoardColumn } from "./work-board-column";
import { filterWorkItems } from "./work-board-filter";
import { fieldClass, WorkItemForm } from "./work-board-form";

export function WorkBoard() {
  const { t } = useTranslation();
  const compact = useMediaQuery("(max-width: 900px)");
  const [query, setQuery] = useState("");
  const [archived, setArchived] = useState(false);
  const [source, setSource] = useState<WorkItemSourceKind | "all">("all");
  const [priority, setPriority] = useState<WorkItemPriority | "all">("all");
  const [stageFilter, setStageFilter] = useState<WorkItemStage | "all">("all");
  const [project, setProject] = useState<string>("all");
  const [compactStage, setCompactStage] = useState<WorkItemStage>("inbox");
  const [creating, setCreating] = useState(false);
  const [editing, setEditing] = useState<WorkItem | null>(null);

  const { archive, create, error, items, loading, move, mutations, remove, restore, update } = useWorkBoardActions(archived);

  // The option value stays the stored path so filtering keeps matching; only the label is
  // normalized, because a stored `\\?\` prefix is a Windows API detail, not something to read.
  const projects = useMemo(
    () => [...new Set(items.flatMap((item) => item.projectPath ? [item.projectPath] : []))]
      .sort()
      .map((path) => ({ label: normalizeDisplayPath(path), value: path })),
    [items],
  );
  const visible = useMemo(() => filterWorkItems(items, {
    archived,
    query,
    sourceKinds: source === "all" ? undefined : [source],
    stages: stageFilter === "all" ? undefined : [stageFilter],
    priorities: priority === "all" ? undefined : [priority],
    projectPaths: project === "all" ? undefined : [project],
  }), [archived, items, priority, project, query, source, stageFilter]);

  const drop = (event: DragEvent<HTMLElement>, stage: WorkItemStage) => {
    event.preventDefault();
    const item = items.find((candidate) => candidate.id === event.dataTransfer.getData("text/work-item"));
    if (item) void move(item, stage);
  };

  // An empty column means something different when a filter is narrowing the board than when
  // the board genuinely has nothing in it.
  const filtersActive = Boolean(query.trim()) || source !== "all" || priority !== "all"
    || stageFilter !== "all" || project !== "all";
  const clearFilters = () => {
    setQuery("");
    setSource("all");
    setPriority("all");
    setStageFilter("all");
    setProject("all");
  };
  const stages = compact ? [compactStage] : workItemStages;

  return <section aria-labelledby="todo-board-title" className="ucd-panel flex h-full min-h-0 flex-1 flex-col overflow-hidden rounded-lg" id="todo-board">
    <header className="grid shrink-0 gap-3 border-b border-border p-3 md:p-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-base font-semibold" id="todo-board-title">{t("todoBoard.title")}</h1>
          <p className="text-xs text-muted-foreground">{t("todoBoard.subtitle")}</p>
        </div>
        <div className="flex gap-2">
          <Button onClick={() => setArchived((value) => !value)} size="sm" type="button" variant="outline">
            {archived ? <Inbox aria-hidden="true" /> : <Archive aria-hidden="true" />}
            {archived ? t("todoBoard.active") : t("todoBoard.archived")}
          </Button>
          <Button onClick={() => { setEditing(null); setCreating((value) => !value); }} size="sm" type="button">
            <Plus aria-hidden="true" />{t("todoBoard.new")}
          </Button>
        </div>
      </div>
      {creating ? <WorkItemForm mutation={mutations.get(CREATE_MUTATION_KEY)} onCancel={() => setCreating(false)} onSubmit={(input) => void create(input, () => setCreating(false))} submitLabel={t("todoBoard.create")} /> : null}
      {editing ? <WorkItemForm item={editing} mutation={mutations.get(editing.id)} onCancel={() => setEditing(null)} onSubmit={(input) => void update(editing, input, () => setEditing(null))} submitLabel={t("todoBoard.save")} /> : null}

      <label className="relative block">
        <Search aria-hidden="true" className="absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
        <span className="sr-only">{t("todoBoard.search")}</span>
        <input className={`${fieldClass} w-full pl-9`} onChange={(event) => setQuery(event.target.value)} placeholder={t("todoBoard.search")} value={query} />
      </label>
      <div className="grid gap-2 rounded-md border border-border/70 bg-muted/20 p-2">
        <div className="flex items-center justify-between gap-2">
          <span className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">{t("todoBoard.filters")}</span>
          {filtersActive ? (
            <Button className="h-6 px-2 text-[11px]" onClick={clearFilters} size="sm" type="button" variant="ghost">
              <FilterX aria-hidden="true" className="h-3 w-3" />{t("todoBoard.clearFilters")}
            </Button>
          ) : null}
        </div>
        <div className="grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
          <select aria-label={t("todoBoard.sourceFilter")} className={fieldClass} onChange={(event) => setSource(event.target.value as WorkItemSourceKind | "all")} value={source}><option value="all">{t("todoBoard.source.all")}</option>{workItemSourceKinds.map((kind) => <option key={kind} value={kind}>{t(`todoBoard.source.${kind}`)}</option>)}</select>
          <select aria-label={t("todoBoard.priorityFilter")} className={fieldClass} onChange={(event) => setPriority(event.target.value as WorkItemPriority | "all")} value={priority}><option value="all">{t("todoBoard.priority.all")}</option>{workItemPriorities.map((kind) => <option key={kind} value={kind}>{t(`todoBoard.priority.${kind}`)}</option>)}</select>
          <select aria-label={t("todoBoard.stageFilter")} className={fieldClass} onChange={(event) => setStageFilter(event.target.value as WorkItemStage | "all")} value={stageFilter}><option value="all">{t("todoBoard.stage.all")}</option>{workItemStages.map((stage) => <option key={stage} value={stage}>{t(`todoBoard.stage.${stage}`)}</option>)}</select>
          <select aria-label={t("todoBoard.projectFilter")} className={fieldClass} onChange={(event) => setProject(event.target.value)} value={project}><option value="all">{t("todoBoard.project.all")}</option>{projects.map((entry) => <option key={entry.value} value={entry.value}>{entry.label}</option>)}</select>
        </div>
        {filtersActive ? <p className="text-[11px] text-muted-foreground" role="status">{t("todoBoard.filtersActive", { count: visible.length })}</p> : null}
      </div>
      {compact ? <select aria-label={t("todoBoard.stage")} className={fieldClass} onChange={(event) => setCompactStage(event.target.value as WorkItemStage)} value={compactStage}>{workItemStages.map((stage) => <option key={stage} value={stage}>{t(`todoBoard.stage.${stage}`)}</option>)}</select> : null}
    </header>
    {error ? <p className="m-3 rounded border border-destructive/50 bg-destructive/10 p-2 text-sm text-destructive" role="alert">{error}</p> : null}
    {loading && !items.length
      ? <div className="grid flex-1 place-items-center"><Loader2 aria-label={t("todoBoard.loading")} className="animate-spin" /></div>
      : <div className="flex min-h-0 flex-1 gap-3 overflow-x-auto p-3">
          {stages.map((stage) => (
            <WorkBoardColumn
              filtersActive={filtersActive}
              items={visible.filter((item) => item.stage === stage).sort((left, right) => left.rank - right.rank)}
              key={stage}
              mutations={mutations.registry}
              onArchive={(item) => void archive(item)}
              onDelete={(item) => void remove(item)}
              onDismissError={(item) => mutations.clear(item.id)}
              onDrop={drop}
              onEdit={(item) => { setCreating(false); setEditing(item); }}
              onMove={(item, target) => void move(item, target)}
              onRestore={(item) => void restore(item)}
              stage={stage}
            />
          ))}
        </div>}
  </section>;
}
