import { Activity, Plus, RefreshCw, Settings2, Workflow } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { orderByAgentPriority } from "../../lib/agent-display-order";
import type { AgentService } from "../../services/agent-service";
import { agentService } from "../../services/runtime-agent-client";
import { managedCliAgentIds, type AgentRegistryEntry, type ManagedCliAgentId } from "../../types/agent";
import type { PromptHook, PromptHookCategory, PromptHookMutationInput } from "../../types/prompt-hook";
import { PageHeader } from "./page-parts";
import { PromptHookCardList } from "./prompt-hooks/prompt-hook-card-list";
import { PromptHookDetailPanel } from "./prompt-hooks/prompt-hook-detail-panel";
import { PromptHookDialogs, type PromptHookDialogState } from "./prompt-hooks/prompt-hook-dialogs";
import { PromptHookFilterToolbar } from "./prompt-hooks/prompt-hook-filter-toolbar";
import { PromptHookInventorySummary } from "./prompt-hooks/prompt-hook-inventory-summary";
import { PromptHookTracePanel } from "./prompt-hooks/prompt-hook-trace-panel";
import {
  defaultPromptHookFilters,
  filterPromptHooks,
  groupPromptHooks,
  promptHookCategoryOrder,
  type PromptHookFilters,
} from "./prompt-hooks/prompt-hook-view-model";

type ManagedAgent = AgentRegistryEntry & { id: ManagedCliAgentId };
type PromptHookView = "management" | "runtime";
const emptyHooks: PromptHook[] = [];

export function PromptHooksPage({ searchTerm, service = agentService }: { searchTerm: string; service?: AgentService }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [view, setView] = useState<PromptHookView>("management");
  const [filters, setFilters] = useState<PromptHookFilters>(defaultPromptHookFilters);
  const [expandedCategories, setExpandedCategories] = useState(() => new Set(promptHookCategoryOrder));
  const [dialog, setDialog] = useState<PromptHookDialogState>({ mode: null, hook: null, preview: null });
  const [detailHookId, setDetailHookId] = useState<string | null>(null);

  const agentsQuery = useQuery({ queryKey: ["agents", "prompt-hooks"], queryFn: () => service.listAgents() });
  const hooksQuery = useQuery({ queryKey: ["prompt-hooks"], queryFn: () => service.listPromptHooks() });
  const tracesQuery = useQuery({
    enabled: view === "runtime",
    queryKey: ["prompt-hook-traces"],
    queryFn: () => service.listPromptHookTraces(20),
  });
  const rawHooks = hooksQuery.data?.hooks ?? emptyHooks;
  const userHookIds = useMemo(() => rawHooks.filter((hook) => hook.source === "user").map((hook) => hook.id), [rawHooks]);
  const lifecycleQueries = useQueries({
    queries: userHookIds.map((hookId) => ({
      queryKey: ["prompt-hook-history", hookId],
      queryFn: () => service.getPromptHookVersionHistory(hookId),
    })),
  });
  const hooks = useMemo(() => withLifecycleState(rawHooks, userHookIds, lifecycleQueries), [lifecycleQueries, rawHooks, userHookIds]);
  const agents = useMemo(
    () => orderByAgentPriority((agentsQuery.data ?? []).filter(isManagedAgent), (item) => item.id),
    [agentsQuery.data],
  );
  const visibleHooks = useMemo(() => filterPromptHooks(hooks, filters, searchTerm), [filters, hooks, searchTerm]);
  const groups = useMemo(() => groupPromptHooks(visibleHooks), [visibleHooks]);
  const groupCategoriesKey = groups.map((group) => group.category).join(",");
  const groupResetKey = JSON.stringify([filters, searchTerm, groupCategoriesKey]);
  const detailHook = hooks.find((hook) => hook.id === detailHookId) ?? null;
  const stats = hooksQuery.data?.stats ?? { total: 0, enabled: 0, builtin: 0, user: 0 };

  useEffect(() => {
    const categories = groupCategoriesKey ? groupCategoriesKey.split(",") as PromptHookCategory[] : [];
    setExpandedCategories(new Set(categories));
  }, [groupCategoriesKey]);

  const invalidate = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["prompt-hooks"] }),
      queryClient.invalidateQueries({ queryKey: ["prompt-hook-history"] }),
      queryClient.invalidateQueries({ queryKey: ["prompt-hook-traces"] }),
    ]);
  };
  const enabledMutation = useMutation({
    mutationFn: ({ hook, value }: { hook: PromptHook; value: boolean }) => service.setPromptHookEnabled(hook.id, value),
    onSuccess: () => void invalidate(),
  });
  const bindingMutation = useMutation({
    mutationFn: ({ hook, agentIds }: { hook: PromptHook; agentIds: ManagedCliAgentId[] }) => service.setPromptHookCliBindings(hook.id, agentIds),
    onSuccess: () => void invalidate(),
  });
  const createMutation = useMutation({
    mutationFn: (input: PromptHookMutationInput) => service.createPromptHook(input),
    onSuccess: () => { closeDialog(); void invalidate(); },
  });
  const deleteMutation = useMutation({
    mutationFn: (hook: PromptHook) => service.deletePromptHook(hook.id),
    onSuccess: () => { closeDialog(); void invalidate(); },
  });
  const previewMutation = useMutation({
    mutationFn: (hook: PromptHook) => service.previewPromptHook({ hookId: hook.id, agentId: firstAgentId(agentsQuery.data) }),
    onSuccess: (preview) => { setDialog({ mode: null, hook: null, preview }); void invalidate(); },
  });
  const assemblyPreviewMutation = useMutation({
    mutationFn: () => service.previewPromptAssembly({ agentId: firstAgentId(agentsQuery.data), sampleInput: t("promptHooks.preview.sample") }),
    onSuccess: (preview) => { setDialog({ mode: null, hook: null, preview }); void invalidate(); },
  });

  function toggleAgentBinding(hook: PromptHook, agentId: ManagedCliAgentId, checked: boolean) {
    const agentIds = checked
      ? Array.from(new Set([...hook.cliBindings, agentId]))
      : hook.cliBindings.filter((id) => id !== agentId);
    bindingMutation.mutate({ hook, agentIds });
  }
  function openDialog(next: PromptHookDialogState) {
    createMutation.reset();
    deleteMutation.reset();
    setDialog(next);
  }
  function closeDialog() {
    createMutation.reset();
    deleteMutation.reset();
    setDialog({ mode: null, hook: null, preview: null });
  }
  function requestDelete(hook: PromptHook) {
    setDetailHookId(null);
    openDialog({ mode: "delete", hook, preview: null });
  }

  return (
    <div className="space-y-4">
      <PageHeader
        actions={<HeaderActions fetching={hooksQuery.isFetching} onCreate={() => openDialog({ mode: "create", hook: null, preview: null })} onRefresh={() => void hooksQuery.refetch()} />}
        description={t("promptHooks.description")}
        icon={Workflow}
        title={t("promptHooks.title")}
      />
      <ViewTabs view={view} onChange={setView} />
      {view === "management" ? (
        <>
          <PromptHookInventorySummary stats={stats} visible={visibleHooks.length} />
          <PromptHookFilterToolbar agents={agents} filters={filters} onChange={setFilters} />
          {hooksQuery.isLoading ? <div className="ucd-panel rounded-lg p-4 text-sm text-muted-foreground">{t("promptHooks.loading")}</div> : null}
          <PromptHookCardList
            busyHookId={enabledMutation.variables?.hook.id ?? bindingMutation.variables?.hook.id ?? null}
            expandedCategories={expandedCategories}
            hooks={visibleHooks}
            onDelete={requestDelete}
            onOpen={(hook) => setDetailHookId(hook.id)}
            onPreview={(hook) => previewMutation.mutate(hook)}
            onToggleCategory={(category) => setExpandedCategories((current) => toggleSetValue(current, category))}
            onToggleEnabled={(hook, value) => enabledMutation.mutate({ hook, value })}
            resetKey={groupResetKey}
          />
        </>
      ) : (
        <RuntimeRecords
          assemblyPending={assemblyPreviewMutation.isPending}
          traceError={tracesQuery.isError}
          traceFetching={tracesQuery.isFetching}
          traces={tracesQuery.data ?? []}
          onPreview={() => assemblyPreviewMutation.mutate()}
          onRefresh={() => void tracesQuery.refetch()}
        />
      )}
      <PromptHookDialogs
        error={errorMessage(createMutation.error ?? deleteMutation.error)}
        state={dialog}
        onClose={closeDialog}
        onCreate={(input) => createMutation.mutate(input)}
        onDelete={(hook) => deleteMutation.mutate(hook)}
      />
      {detailHook ? (
        <PromptHookDetailPanel
          agents={agents}
          hook={detailHook}
          onChanged={() => void invalidate()}
          onClose={() => setDetailHookId(null)}
          onDelete={requestDelete}
          onPreview={(hook) => previewMutation.mutate(hook)}
          onToggleAgent={toggleAgentBinding}
          onToggleEnabled={(hook, value) => enabledMutation.mutate({ hook, value })}
          service={service}
        />
      ) : null}
    </div>
  );
}

function HeaderActions({ fetching, onCreate, onRefresh }: { fetching: boolean; onCreate: () => void; onRefresh: () => void }) {
  const { t } = useTranslation();
  return <><Button disabled={fetching} onClick={onRefresh} variant="outline"><RefreshCw className={fetching ? "animate-spin" : ""} aria-hidden="true" />{t("promptHooks.refresh")}</Button><Button onClick={onCreate}><Plus aria-hidden="true" />{t("promptHooks.createHook")}</Button></>;
}

function ViewTabs({ view, onChange }: { view: PromptHookView; onChange: (view: PromptHookView) => void }) {
  const { t } = useTranslation();
  return <div className="inline-flex rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-1" role="tablist">{(["management", "runtime"] as const).map((item) => <button aria-selected={view === item} className={`flex h-9 items-center gap-2 rounded-md px-3 text-sm font-medium focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring ${view === item ? "bg-background text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground"}`} key={item} onClick={() => onChange(item)} role="tab" type="button">{item === "management" ? <Settings2 aria-hidden="true" /> : <Activity aria-hidden="true" />}{t(`promptHooks.views.${item}`)}</button>)}</div>;
}

function RuntimeRecords({ assemblyPending, traceError, traceFetching, traces, onPreview, onRefresh }: { assemblyPending: boolean; traceError: boolean; traceFetching: boolean; traces: Awaited<ReturnType<AgentService["listPromptHookTraces"]>>; onPreview: () => void; onRefresh: () => void }) {
  const { t } = useTranslation();
  return <div className="space-y-4"><div className="ucd-panel flex flex-wrap items-center justify-between gap-3 rounded-lg p-4"><p className="text-sm text-muted-foreground">{t("promptHooks.runtime.description")}</p><div className="flex gap-2"><Button disabled={traceFetching} onClick={onRefresh} variant="outline"><RefreshCw className={traceFetching ? "animate-spin" : ""} aria-hidden="true" />{t("promptHooks.refresh")}</Button><Button disabled={assemblyPending} onClick={onPreview}>{t("promptHooks.previewAssembly")}</Button></div></div>{traceError ? <div className="rounded-md border px-3 py-2 text-sm ucd-status-danger">{t("promptHooks.trace.loadError")}</div> : null}<PromptHookTracePanel traces={traces} /></div>;
}

function withLifecycleState(rawHooks: PromptHook[], userHookIds: string[], queries: { data?: Awaited<ReturnType<AgentService["getPromptHookVersionHistory"]>> }[]) {
  const histories = new Map(userHookIds.map((hookId, index) => [hookId, queries[index]?.data]));
  return rawHooks.map((hook) => {
    const history = histories.get(hook.id);
    return history ? { ...hook, publishedVersion: history.publishedVersion ?? null, hasDraft: Boolean(history.draft), draftRevision: history.draft?.revision ?? null } : hook;
  });
}

function toggleSetValue<Value>(current: ReadonlySet<Value>, value: Value) {
  const next = new Set(current);
  if (next.has(value)) next.delete(value); else next.add(value);
  return next;
}

function isManagedAgent(agent: AgentRegistryEntry): agent is ManagedAgent { return isManagedCliAgentId(agent.id); }
function isManagedCliAgentId(agentId: string): agentId is ManagedCliAgentId { return managedCliAgentIds.some((id) => id === agentId); }
function firstAgentId(agents: AgentRegistryEntry[] | undefined): ManagedCliAgentId { return orderByAgentPriority(agents?.filter(isManagedAgent) ?? [], (item) => item.id)[0]?.id ?? "claude-code"; }
function errorMessage(error: unknown) { return !error ? null : error instanceof Error ? error.message : String(error); }
