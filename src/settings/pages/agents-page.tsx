import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Activity, Bot, CircleAlert, Play, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../../components/agent-brand-icon";
import { Button } from "../../components/ui/button";
import { agentService } from "../../services/runtime-agent-client";
import type { AgentRegistryEntry, InteractionMode, SessionDetails, WorkflowState } from "../../types/agent";
import { getAgentVisualIdentity } from "../../lib/agent-visual-identity";
import { PageHeader, SectionPanel } from "./page-parts";
import { AgentEditDialog } from "./agents/agent-edit-dialog";
import { AgentMemoryPanel } from "./agents/agent-memory-panel";
import { AgentRuntimeCard } from "./agents/agent-runtime-card";
import { ApiAgentRegistrationPanel } from "./agents/api-agent-registration-panel";
import type { SettingsPageContext } from "../settings-pages";

interface AgentsOverview {
  agents: AgentRegistryEntry[];
  workflow: WorkflowState;
  sessionDetails: SessionDetails;
}

const emptyAgents: AgentRegistryEntry[] = [];
const overviewKey = (filter: string) => ["agents", "overview", filter] as const;

export function AgentsPage({ searchTerm, onNavigate }: SettingsPageContext) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [filter, setFilter] = useState("");
  const [appliedFilter, setAppliedFilter] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [editingAgent, setEditingAgent] = useState<AgentRegistryEntry | null>(null);

  const overviewQuery = useQuery({
    queryKey: overviewKey(appliedFilter),
    queryFn: async (): Promise<AgentsOverview> => {
      const [agents, workflow, sessionDetails] = await Promise.all([
        agentService.listAgents(appliedFilter || undefined),
        agentService.getWorkflowState(),
        agentService.getSessionDetails(),
      ]);
      return { agents, workflow, sessionDetails };
    },
  });
  const agents = overviewQuery.data?.agents ?? emptyAgents;
  const workflow = overviewQuery.data?.workflow ?? null;
  const sessionDetails = overviewQuery.data?.sessionDetails ?? null;

  const selectMutation = useMutation({
    mutationFn: ({ agent, mode }: { agent: AgentRegistryEntry; mode: InteractionMode }) => agentService.selectAgent(agent.id, mode),
    onSuccess: async (_result, input) => {
      setNotice(t("agents.notice.selected", { agent: input.agent.displayName, mode: t(`agents.mode.${input.mode}`) }));
      await queryClient.invalidateQueries({ queryKey: ["agents", "overview"] });
    },
  });
  const launchMutation = useMutation({
    mutationFn: async () => {
      if (workflow?.activeAgentId && workflow.activeInteractionMode === "browser") {
        const readiness = await agentService.checkBrowserReadiness(workflow.activeAgentId);
        if (!readiness.ready) throw new Error(readiness.reason ?? t("agents.error.browserNotReady"));
      }
      return agentService.launchActiveWorkflow();
    },
    onSuccess: async (result) => {
      setNotice(result.message);
      await queryClient.invalidateQueries({ queryKey: ["agents", "overview"] });
    },
  });
  const deleteMutation = useMutation({
    mutationFn: (agentId: string) => agentService.deleteApiAgent(agentId),
    onSuccess: async () => queryClient.invalidateQueries({ queryKey: ["agents", "overview"] }),
    onError: (reason) => setError(reason instanceof Error ? reason.message : String(reason)),
  });

  const filteredAgents = useMemo(() => {
    const query = searchTerm.trim().toLowerCase();
    return query
      ? agents.filter((agent) => [agent.displayName, agent.provider, ...agent.capabilityTags].some((value) => value.toLowerCase().includes(query)))
      : agents;
  }, [agents, searchTerm]);
  const activeAgent = agents.find((agent) => agent.id === workflow?.activeAgentId) ?? null;
  const visibleError = error ?? (overviewQuery.error instanceof Error ? overviewQuery.error.message : overviewQuery.error ? String(overviewQuery.error) : null);

  function applyFilter() {
    const next = filter.trim();
    setError(null);
    if (next === appliedFilter) void overviewQuery.refetch();
    else setAppliedFilter(next);
  }

  function selectAgent(agent: AgentRegistryEntry, mode: InteractionMode) {
    setError(null);
    if (!(["available", "unknown"] as const).some((state) => state === agent.availabilityState)) {
      setError(agent.unavailableReason ?? t("agents.error.notAvailable", { agent: agent.displayName }));
      return;
    }
    if (!agent.supportedInteractionModes.includes(mode)) {
      setError(t("agents.error.supportedModes", { agent: agent.displayName, modes: agent.supportedInteractionModes.map((item) => t(`agents.mode.${item}`)).join(", ") }));
      return;
    }
    void selectMutation.mutateAsync({ agent, mode }).catch((reason) => setError(reason instanceof Error ? reason.message : String(reason)));
  }

  function deleteAgent(agent: AgentRegistryEntry) {
    if (window.confirm(t("agents.delete.confirm", { agent: agent.displayName }))) {
      void deleteMutation.mutateAsync(agent.id).catch(() => undefined);
    }
  }

  return (
    <div className="space-y-4">
      <PageHeader actions={<Button disabled={overviewQuery.isFetching} onClick={() => void overviewQuery.refetch()} variant="outline"><RefreshCw className="h-4 w-4" />{overviewQuery.isFetching ? t("agents.refreshing") : t("agents.refresh")}</Button>} description={t("agents.description")} icon={Bot} title={t("agents.title")} />
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_340px]">
        <div className="space-y-4">
          <SectionPanel title={t("agents.filter.title")} description={t("agents.filter.description")}>
            <div className="flex flex-wrap gap-2"><input className="ucd-input h-9 min-w-56 flex-1 rounded px-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => setFilter(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter") applyFilter(); }} placeholder={t("agents.filter.placeholder")} value={filter} /><Button onClick={applyFilter} variant="outline">{t("agents.filter.apply")}</Button></div>
          </SectionPanel>
          <ApiAgentRegistrationPanel onCreated={async (agent) => { setNotice(t("agents.registerApiAgent.success", { agent: agent.displayName })); await queryClient.invalidateQueries({ queryKey: ["agents", "overview"] }); }} onError={setError} />
          <div className="grid gap-4 lg:grid-cols-2">
            {filteredAgents.map((agent) => <AgentRuntimeCard active={workflow?.activeAgentId === agent.id} activeMode={workflow?.activeAgentId === agent.id ? workflow.activeInteractionMode : null} agent={agent} key={agent.id} onDelete={deleteAgent} onEdit={setEditingAgent} onManageConfigurations={(agentId) => onNavigate("agent-configurations", { cliConfigAgentId: agentId })} onSelect={selectAgent} />)}
          </div>
        </div>

        <div className="space-y-4">
          <SectionPanel title={t("agents.details.title")} description={t("agents.details.description")}>
            <dl className="grid gap-4 text-sm">
              <div><dt className="text-xs uppercase text-muted-foreground">{t("agents.details.activeAgent")}</dt><dd className="mt-1 flex min-w-0 items-center gap-2 font-medium">{activeAgent ? <span className={`flex h-6 w-6 shrink-0 items-center justify-center rounded border ${getAgentVisualIdentity(activeAgent.id).tone}`}><AgentBrandIcon agentId={activeAgent.id} className="h-3.5 w-3.5" /></span> : null}<span className="truncate">{activeAgent?.displayName ?? t("agents.details.noneSelected")}</span></dd></div>
              <div><dt className="text-xs uppercase text-muted-foreground">{t("agents.details.interactionMode")}</dt><dd className="mt-1 font-medium">{workflow?.activeInteractionMode ? t(`agents.mode.${workflow.activeInteractionMode}`) : t("agents.details.notSelected")}</dd></div>
              <div><dt className="text-xs uppercase text-muted-foreground">{t("agents.details.lifecycle")}</dt><dd className="mt-1 font-medium">{workflow?.lifecycleState ?? t("agents.status.idle")}</dd></div>
              <div><dt className="text-xs uppercase text-muted-foreground">{t("agents.details.intent")}</dt><dd className="mt-1 text-muted-foreground">{workflow?.intent ?? t("agents.details.defaultIntent")}</dd></div>
            </dl>
            {visibleError ? <div className="mt-5 flex gap-2 rounded-md border p-3 text-sm ucd-status-warning"><CircleAlert className="mt-0.5 h-4 w-4 shrink-0" /><span>{visibleError}</span></div> : null}
            {notice ? <div className="mt-5 rounded-md border p-3 text-sm ucd-status-success">{notice}</div> : null}
            <Button className="mt-5 w-full" disabled={!activeAgent || launchMutation.isPending} onClick={() => void launchMutation.mutateAsync().catch((reason) => setError(reason instanceof Error ? reason.message : String(reason)))}><Play className="h-4 w-4" />{t("agents.launch")}</Button>
            <div className="mt-5 border-t border-border pt-4"><div className="mb-2 flex items-center gap-2 text-sm font-medium"><Activity className="h-4 w-4 text-muted-foreground" />{t("agents.details.session")}</div>{sessionDetails ? <dl className="grid gap-2 text-xs text-muted-foreground"><div className="flex justify-between gap-3"><dt>{t("agents.details.adapter")}</dt><dd className="font-medium text-foreground">{sessionDetails.adapter}</dd></div><div className="flex justify-between gap-3"><dt>{t("agents.details.runtime")}</dt><dd className="font-medium text-foreground">{sessionDetails.details.runtime ?? "desktop"}</dd></div></dl> : null}</div>
          </SectionPanel>
          <AgentMemoryPanel agentId={activeAgent?.id ?? null} />
        </div>
      </div>
      {editingAgent ? <AgentEditDialog agent={editingAgent} onClose={() => setEditingAgent(null)} onSaved={async () => { setEditingAgent(null); setNotice(t("agents.edit.success")); await queryClient.invalidateQueries({ queryKey: ["agents", "overview"] }); }} /> : null}
    </div>
  );
}
