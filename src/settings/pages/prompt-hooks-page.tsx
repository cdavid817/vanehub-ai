import { Play, Plus, RefreshCw, Workflow } from "lucide-react";
import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import type { AgentService } from "../../services/agent-service";
import { agentService } from "../../services/runtime-agent-client";
import { managedCliAgentIds, type AgentRegistryEntry, type ManagedCliAgentId } from "../../types/agent";
import type { PromptHook, PromptHookMutationInput } from "../../types/prompt-hook";
import { PageHeader } from "./page-parts";
import { PromptHookCardList } from "./prompt-hooks/prompt-hook-card-list";
import { PromptHookDialogs, type PromptHookDialogState } from "./prompt-hooks/prompt-hook-dialogs";
import { PromptHookFilterToolbar } from "./prompt-hooks/prompt-hook-filter-toolbar";
import { PromptHookStatsCards } from "./prompt-hooks/prompt-hook-stats-cards";
import { PromptHookTracePanel } from "./prompt-hooks/prompt-hook-trace-panel";

type ManagedAgent = AgentRegistryEntry & { id: ManagedCliAgentId };
const emptyHooks: PromptHook[] = [];

export function PromptHooksPage({ searchTerm, service = agentService }: { searchTerm: string; service?: AgentService }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [category, setCategory] = useState("__all__");
  const [source, setSource] = useState("__all__");
  const [enabled, setEnabled] = useState("__all__");
  const [agent, setAgent] = useState("__all__");
  const [query, setQuery] = useState(searchTerm);
  const [dialog, setDialog] = useState<PromptHookDialogState>({ mode: null, hook: null, preview: null });

  const agentsQuery = useQuery({ queryKey: ["agents", "prompt-hooks"], queryFn: () => service.listAgents() });
  const hooksQuery = useQuery({ queryKey: ["prompt-hooks"], queryFn: () => service.listPromptHooks() });
  const tracesQuery = useQuery({ queryKey: ["prompt-hook-traces"], queryFn: () => service.listPromptHookTraces(20) });

  const invalidate = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["prompt-hooks"] }),
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
    onSuccess: () => {
      setDialog({ mode: null, hook: null, preview: null });
      void invalidate();
    },
  });
  const updateMutation = useMutation({
    mutationFn: ({ hook, input }: { hook: PromptHook; input: PromptHookMutationInput }) =>
      service.updatePromptHook(hook.id, { ...input, version: hook.version }),
    onSuccess: () => {
      setDialog({ mode: null, hook: null, preview: null });
      void invalidate();
    },
  });
  const deleteMutation = useMutation({
    mutationFn: (hook: PromptHook) => service.deletePromptHook(hook.id),
    onSuccess: () => {
      setDialog({ mode: null, hook: null, preview: null });
      void invalidate();
    },
  });
  const previewMutation = useMutation({
    mutationFn: (hook: PromptHook) => service.previewPromptHook({ hookId: hook.id, agentId: firstAgentId(agentsQuery.data) }),
    onSuccess: (preview) => {
      setDialog({ mode: null, hook: null, preview });
      void invalidate();
    },
  });
  const assemblyPreviewMutation = useMutation({
    mutationFn: () => service.previewPromptAssembly({ agentId: firstAgentId(agentsQuery.data), sampleInput: t("promptHooks.preview.sample") }),
    onSuccess: (preview) => {
      setDialog({ mode: null, hook: null, preview });
      void invalidate();
    },
  });

  const hooks = hooksQuery.data?.hooks ?? emptyHooks;
  const stats = hooksQuery.data?.stats ?? { total: 0, enabled: 0, builtin: 0, user: 0 };
  const agents = useMemo(() => (agentsQuery.data ?? []).filter(isManagedAgent), [agentsQuery.data]);
  const categories = useMemo(() => ["__all__", ...Array.from(new Set(hooks.map((hook) => hook.category)))], [hooks]);
  const visibleHooks = useMemo(() => {
    const needle = `${query} ${searchTerm}`.trim().toLowerCase();
    return hooks.filter((hook) => {
      if (category !== "__all__" && hook.category !== category) return false;
      if (source !== "__all__" && hook.source !== source) return false;
      if (enabled === "enabled" && !hook.enabled) return false;
      if (enabled === "disabled" && hook.enabled) return false;
      if (agent !== "__all__" && !hook.cliBindings.includes(agent as ManagedCliAgentId)) return false;
      if (!needle) return true;
      return `${hook.id} ${hook.name} ${hook.description} ${hook.category} ${hook.stage} ${hook.source}`.toLowerCase().includes(needle);
    });
  }, [agent, category, enabled, hooks, query, searchTerm, source]);

  function toggleAgentBinding(hook: PromptHook, agentId: string, checked: boolean) {
    if (!isManagedCliAgentId(agentId)) return;
    const agentIds = checked
      ? Array.from(new Set([...hook.cliBindings, agentId]))
      : hook.cliBindings.filter((id) => id !== agentId);
    bindingMutation.mutate({ hook, agentIds });
  }

  function resetDialogErrors() {
    createMutation.reset();
    updateMutation.reset();
    deleteMutation.reset();
  }

  function openDialog(next: PromptHookDialogState) {
    resetDialogErrors();
    setDialog(next);
  }

  function closeDialog() {
    resetDialogErrors();
    setDialog({ mode: null, hook: null, preview: null });
  }

  const dialogError = errorMessage(createMutation.error ?? updateMutation.error ?? deleteMutation.error);

  return (
    <div className="space-y-4">
      <PageHeader
        actions={
          <>
            <Button disabled={hooksQuery.isFetching} onClick={() => void hooksQuery.refetch()} variant="outline">
              <RefreshCw className={hooksQuery.isFetching ? "animate-spin" : ""} aria-hidden="true" />
              {t("promptHooks.refresh")}
            </Button>
            <Button disabled={assemblyPreviewMutation.isPending} onClick={() => assemblyPreviewMutation.mutate()} variant="outline">
              <Play aria-hidden="true" />
              {t("promptHooks.previewAssembly")}
            </Button>
            <Button onClick={() => openDialog({ mode: "create", hook: null, preview: null })}>
              <Plus aria-hidden="true" />
              {t("promptHooks.createHook")}
            </Button>
          </>
        }
        description={t("promptHooks.description")}
        icon={Workflow}
        title={t("promptHooks.title")}
      />
      <PromptHookStatsCards stats={stats} />
      <PromptHookFilterToolbar
        agent={agent}
        agents={agents}
        categories={categories}
        category={category}
        enabled={enabled}
        query={query}
        source={source}
        onAgentChange={setAgent}
        onCategoryChange={setCategory}
        onEnabledChange={setEnabled}
        onQueryChange={setQuery}
        onSourceChange={setSource}
      />
      {hooksQuery.isLoading ? <div className="ucd-panel rounded-lg p-4 text-sm text-muted-foreground">{t("promptHooks.loading")}</div> : null}
      <PromptHookCardList
        agents={agents}
        busyHookId={enabledMutation.variables?.hook.id ?? bindingMutation.variables?.hook.id ?? null}
        hooks={visibleHooks}
        onDelete={(hook) => openDialog({ mode: "delete", hook, preview: null })}
        onEdit={(hook) => openDialog({ mode: "edit", hook, preview: null })}
        onPreview={(hook) => previewMutation.mutate(hook)}
        onToggleAgent={toggleAgentBinding}
        onToggleEnabled={(hook, value) => enabledMutation.mutate({ hook, value })}
        resetKey={JSON.stringify([agent, category, enabled, query, searchTerm, source])}
      />
      <div className="ucd-panel rounded-lg p-3 text-sm text-muted-foreground">
        {t("promptHooks.showing", { visible: visibleHooks.length, total: hooks.length })}
      </div>
      <PromptHookTracePanel traces={tracesQuery.data ?? []} />
      <PromptHookDialogs
        error={dialogError}
        state={dialog}
        onClose={closeDialog}
        onCreate={(input) => createMutation.mutate(input)}
        onDelete={(hook) => deleteMutation.mutate(hook)}
        onUpdate={(hook, input) => updateMutation.mutate({ hook, input })}
      />
    </div>
  );
}

function isManagedAgent(agent: AgentRegistryEntry): agent is ManagedAgent {
  return isManagedCliAgentId(agent.id);
}

function isManagedCliAgentId(agentId: string): agentId is ManagedCliAgentId {
  return managedCliAgentIds.some((id) => id === agentId);
}

function firstAgentId(agents: AgentRegistryEntry[] | undefined): ManagedCliAgentId {
  return agents?.find(isManagedAgent)?.id ?? "codex-cli";
}

function errorMessage(error: unknown) {
  if (!error) return null;
  return error instanceof Error ? error.message : String(error);
}
