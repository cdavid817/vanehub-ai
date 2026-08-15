import { Archive, Inbox, Loader2, Plus, Search } from "lucide-react";
import { useCallback, useEffect, useMemo, useState, type DragEvent, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { useMediaQuery } from "../hooks/use-media-query";
import { workBoardService } from "../services/runtime-work-board-client";
import type { WorkItem, WorkItemPriority, WorkItemSourceKind, WorkItemStage } from "../types/work-board";
import { workItemPriorities, workItemSourceKinds, workItemStages } from "../types/work-board";
import { WorkBoardCard } from "./work-board-card";
import { filterWorkItems } from "./work-board-filter";

const fieldClass = "ucd-input rounded-md px-3 py-2 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

export function WorkBoard() {
  const { t } = useTranslation();
  const compact = useMediaQuery("(max-width: 900px)");
  const [items, setItems] = useState<WorkItem[]>([]);
  const [query, setQuery] = useState("");
  const [archived, setArchived] = useState(false);
  const [source, setSource] = useState<WorkItemSourceKind | "all">("all");
  const [priority, setPriority] = useState<WorkItemPriority | "all">("all");
  const [stageFilter, setStageFilter] = useState<WorkItemStage | "all">("all");
  const [project, setProject] = useState<string>("all");
  const [compactStage, setCompactStage] = useState<WorkItemStage>("inbox");
  const [creating, setCreating] = useState(false);
  const [editing, setEditing] = useState<WorkItem | null>(null);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setBusy(true);
    try { setItems(await workBoardService.listWorkItems({ archived })); setError(null); }
    catch (reason: unknown) { setError(reason instanceof Error ? reason.message : String(reason)); }
    finally { setBusy(false); }
  }, [archived]);
  useEffect(() => { void load(); }, [load]);

  const projects = useMemo(() => [...new Set(items.flatMap((item) => item.projectPath ? [item.projectPath] : []))].sort(), [items]);
  const visible = useMemo(() => filterWorkItems(items, {
    archived,
    query,
    sourceKinds: source === "all" ? undefined : [source],
    stages: stageFilter === "all" ? undefined : [stageFilter],
    priorities: priority === "all" ? undefined : [priority],
    projectPaths: project === "all" ? undefined : [project],
  }), [archived, items, priority, project, query, source, stageFilter]);

  const perform = async (action: () => Promise<unknown>) => {
    setBusy(true);
    try { await action(); await load(); }
    catch (reason: unknown) { setError(reason instanceof Error ? reason.message : String(reason)); setBusy(false); }
  };
  const move = (item: WorkItem, stage: WorkItemStage) => perform(() => workBoardService.moveWorkItem({ workItemId: item.id, stage }));
  const drop = (event: DragEvent<HTMLElement>, stage: WorkItemStage) => {
    event.preventDefault();
    const item = items.find((candidate) => candidate.id === event.dataTransfer.getData("text/work-item"));
    if (item) void move(item, stage);
  };

  // An empty column means something different when a filter is narrowing the board than when
  // the board genuinely has nothing in it.
  const filtersActive = Boolean(query.trim()) || source !== "all" || priority !== "all"
    || stageFilter !== "all" || project !== "all";
  const stages = compact ? [compactStage] : workItemStages;
  return <section aria-labelledby="todo-board-title" className="ucd-panel flex h-full min-h-0 flex-1 flex-col overflow-hidden rounded-lg" id="todo-board">
    <header className="grid shrink-0 gap-3 border-b border-border p-3 md:p-4">
      <div className="flex flex-wrap items-center justify-between gap-3"><div><h1 className="text-base font-semibold" id="todo-board-title">{t("todoBoard.title")}</h1><p className="text-xs text-muted-foreground">{t("todoBoard.subtitle")}</p></div><div className="flex gap-2"><Button onClick={() => setArchived((value) => !value)} size="sm" type="button" variant="outline">{archived ? <Inbox aria-hidden="true" /> : <Archive aria-hidden="true" />}{archived ? t("todoBoard.active") : t("todoBoard.archived")}</Button><Button onClick={() => { setEditing(null); setCreating((value) => !value); }} size="sm" type="button"><Plus aria-hidden="true" />{t("todoBoard.new")}</Button></div></div>
      {creating ? <WorkItemForm busy={busy} onCancel={() => setCreating(false)} onSubmit={(input) => perform(async () => { await workBoardService.createWorkItem(input); setCreating(false); })} submitLabel={t("todoBoard.create")} /> : null}
      {editing ? <WorkItemForm busy={busy} item={editing} onCancel={() => setEditing(null)} onSubmit={(input) => perform(async () => { await workBoardService.updateWorkItem({ workItemId: editing.id, ...input }); setEditing(null); })} submitLabel={t("todoBoard.save")} /> : null}
      <div className="grid gap-2 sm:grid-cols-2 xl:grid-cols-[minmax(12rem,1fr)_10rem_10rem_10rem_12rem]">
        <label className="relative"><Search aria-hidden="true" className="absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" /><span className="sr-only">{t("todoBoard.search")}</span><input className={`${fieldClass} w-full pl-9`} onChange={(event) => setQuery(event.target.value)} placeholder={t("todoBoard.search")} value={query} /></label>
        <select aria-label={t("todoBoard.sourceFilter")} className={fieldClass} onChange={(event) => setSource(event.target.value as WorkItemSourceKind | "all")} value={source}><option value="all">{t("todoBoard.source.all")}</option>{workItemSourceKinds.map((kind) => <option key={kind} value={kind}>{t(`todoBoard.source.${kind}`)}</option>)}</select>
        <select aria-label={t("todoBoard.priorityFilter")} className={fieldClass} onChange={(event) => setPriority(event.target.value as WorkItemPriority | "all")} value={priority}><option value="all">{t("todoBoard.priority.all")}</option>{workItemPriorities.map((kind) => <option key={kind} value={kind}>{t(`todoBoard.priority.${kind}`)}</option>)}</select>
        <select aria-label={t("todoBoard.stageFilter")} className={fieldClass} onChange={(event) => setStageFilter(event.target.value as WorkItemStage | "all")} value={stageFilter}><option value="all">{t("todoBoard.stage.all")}</option>{workItemStages.map((stage) => <option key={stage} value={stage}>{t(`todoBoard.stage.${stage}`)}</option>)}</select>
        <select aria-label={t("todoBoard.projectFilter")} className={fieldClass} onChange={(event) => setProject(event.target.value)} value={project}><option value="all">{t("todoBoard.project.all")}</option>{projects.map((path) => <option key={path} value={path}>{path}</option>)}</select>
      </div>
      {compact ? <select aria-label={t("todoBoard.stage")} className={fieldClass} onChange={(event) => setCompactStage(event.target.value as WorkItemStage)} value={compactStage}>{workItemStages.map((stage) => <option key={stage} value={stage}>{t(`todoBoard.stage.${stage}`)}</option>)}</select> : null}
    </header>
    {error ? <p className="m-3 rounded border border-destructive/50 bg-destructive/10 p-2 text-sm text-destructive" role="alert">{error}</p> : null}
    {busy && !items.length ? <div className="grid flex-1 place-items-center"><Loader2 aria-label={t("todoBoard.loading")} className="animate-spin" /></div> : <div className="flex min-h-0 flex-1 gap-3 overflow-x-auto p-3">{stages.map((stage) => <section className="flex min-w-[17rem] flex-1 flex-col rounded-lg border border-border bg-muted/10" key={stage} onDragOver={(event) => event.preventDefault()} onDrop={(event) => drop(event, stage)}><header className="flex items-center justify-between border-b border-border px-3 py-2"><h2 className="text-sm font-semibold">{t(`todoBoard.stage.${stage}`)}</h2><span className="text-xs text-muted-foreground">{visible.filter((item) => item.stage === stage).length}</span></header><div className="grid content-start gap-2 overflow-y-auto p-2">{visible.filter((item) => item.stage === stage).map((item) => <WorkBoardCard item={item} key={item.id} onArchive={() => void perform(() => workBoardService.archiveWorkItem(item.id))} onDelete={() => void perform(() => workBoardService.deleteWorkItem(item.id))} onEdit={() => { setCreating(false); setEditing(item); }} onMove={(target) => void move(item, target)} onRestore={() => void perform(() => workBoardService.restoreWorkItem(item.id))} />)}{!visible.some((item) => item.stage === stage) ? <p className="p-4 text-center text-xs text-muted-foreground">{filtersActive ? t("todoBoard.emptyFiltered") : t("todoBoard.empty")}</p> : null}</div></section>)}</div>}
  </section>;
}

type WorkItemFormValues = { title: string; description: string; priority: WorkItemPriority; projectPath: string | null; dueAt: string | null };

function WorkItemForm({ busy, item, onCancel, onSubmit, submitLabel }: { busy: boolean; item?: WorkItem; onCancel: () => void; onSubmit: (input: WorkItemFormValues) => void; submitLabel: string }) {
  const { t } = useTranslation();
  const submit = (event: FormEvent<HTMLFormElement>) => { event.preventDefault(); const data = new FormData(event.currentTarget); onSubmit({ title: String(data.get("title") ?? ""), description: String(data.get("description") ?? ""), priority: String(data.get("priority") ?? "none") as WorkItemPriority, projectPath: String(data.get("project") ?? "").trim() || null, dueAt: String(data.get("due") ?? "").trim() || null }); };
  return <form className="grid gap-2 rounded-md border border-border bg-muted/10 p-3 md:grid-cols-2" onSubmit={submit}><input aria-label={t("todoBoard.fields.title")} className={fieldClass} defaultValue={item?.title} name="title" placeholder={t("todoBoard.fields.title")} required /><input aria-label={t("todoBoard.fields.project")} className={fieldClass} defaultValue={item?.projectPath ?? ""} name="project" placeholder={t("todoBoard.fields.project")} /><textarea aria-label={t("todoBoard.fields.description")} className={`${fieldClass} md:col-span-2`} defaultValue={item?.description} name="description" placeholder={t("todoBoard.fields.description")} /><select aria-label={t("todoBoard.fields.priority")} className={fieldClass} defaultValue={item?.priority ?? "none"} name="priority">{workItemPriorities.map((value) => <option key={value} value={value}>{t(`todoBoard.priority.${value}`)}</option>)}</select><input aria-label={t("todoBoard.fields.due")} className={fieldClass} defaultValue={item?.dueAt?.slice(0, 10) ?? ""} name="due" type="date" /><div className="flex justify-end gap-2 md:col-span-2"><Button onClick={onCancel} type="button" variant="outline">{t("todoBoard.cancel")}</Button><Button disabled={busy} type="submit">{submitLabel}</Button></div></form>;
}
