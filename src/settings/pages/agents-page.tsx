import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Activity, Bot, CheckCircle2, CircleAlert, Laptop, Play, RefreshCw, Search, Terminal } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { agentService } from "../../services/runtime-agent-client";
import type { AgentRegistryEntry, InteractionMode, SessionDetails, WorkflowState } from "../../types/agent";
import { PageHeader, SectionPanel, StatusPill, TagList } from "./page-parts";

type AgentsOverview = {
  agents: AgentRegistryEntry[];
  workflow: WorkflowState;
  sessionDetails: SessionDetails;
};

const agentsOverviewQueryKey = (capabilityFilter: string) => ["agents", "overview", capabilityFilter] as const;

const modeIcons: Record<InteractionMode, typeof Terminal> = {
  browser: Search,
  "native-desktop": Laptop,
  cli: Terminal,
};

function availabilityTone(agent: AgentRegistryEntry): "success" | "warning" | "muted" {
  if (agent.availabilityState === "available") return "success";
  if (agent.availabilityState === "needs-auth" || agent.availabilityState === "unavailable") return "warning";
  return "muted";
}

function defaultMode(agent: AgentRegistryEntry): InteractionMode {
  return agent.supportedInteractionModes[0] ?? "cli";
}

export function AgentsPage({ searchTerm }: { searchTerm: string }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [capabilityFilter, setCapabilityFilter] = useState("");
  const [appliedCapabilityFilter, setAppliedCapabilityFilter] = useState("");
  const [selectedMode, setSelectedMode] = useState<InteractionMode>("cli");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const agentsOverviewQuery = useQuery({
    queryKey: agentsOverviewQueryKey(appliedCapabilityFilter),
    queryFn: async (): Promise<AgentsOverview> => {
    const [agentList, workflowState] = await Promise.all([
      agentService.listAgents(appliedCapabilityFilter || undefined),
      agentService.getWorkflowState(),
    ]);
      return {
        agents: agentList,
        workflow: workflowState,
        sessionDetails: await agentService.getSessionDetails(),
      };
    },
  });

  const agents = agentsOverviewQuery.data?.agents ?? [];
  const workflow = agentsOverviewQuery.data?.workflow ?? null;
  const sessionDetails = agentsOverviewQuery.data?.sessionDetails ?? null;
  const queryError = agentsOverviewQuery.error instanceof Error ? agentsOverviewQuery.error.message : agentsOverviewQuery.error ? String(agentsOverviewQuery.error) : null;
  const visibleError = error ?? queryError;

  useEffect(() => {
    if (workflow?.activeInteractionMode) {
      setSelectedMode(workflow.activeInteractionMode);
    } else if (agents[0]) {
      setSelectedMode(defaultMode(agents[0]));
    }
  }, [agents, workflow?.activeInteractionMode]);

  const selectAgentMutation = useMutation({
    mutationFn: ({ agent, mode }: { agent: AgentRegistryEntry; mode: InteractionMode }) => agentService.selectAgent(agent.id, mode),
    onSuccess: async (_workflow, { agent, mode }) => {
      setNotice(t("agents.notice.selected", { agent: agent.displayName, mode: t(`agents.mode.${mode}`) }));
      await queryClient.invalidateQueries({ queryKey: ["agents", "overview"] });
    },
  });

  const launchWorkflowMutation = useMutation({
    mutationFn: async () => {
      if (workflow?.activeAgentId && workflow.activeInteractionMode === "browser") {
        const readiness = await agentService.checkBrowserReadiness(workflow.activeAgentId);
        if (!readiness.ready) {
          throw new Error(readiness.reason ?? t("agents.error.browserNotReady"));
        }
      }
      return agentService.launchActiveWorkflow();
    },
    onSuccess: async (result) => {
      setNotice(result.message);
      await queryClient.invalidateQueries({ queryKey: ["agents", "overview"] });
    },
  });

  const filteredAgents = useMemo(() => {
    const query = searchTerm.trim().toLowerCase();
    if (!query) return agents;
    return agents.filter((agent) =>
      [agent.displayName, agent.provider, ...agent.capabilityTags].some((value) => value.toLowerCase().includes(query)),
    );
  }, [agents, searchTerm]);

  const activeAgent = useMemo(
    () => agents.find((agent) => agent.id === workflow?.activeAgentId) ?? null,
    [agents, workflow?.activeAgentId],
  );

  async function handleSelect(agent: AgentRegistryEntry, mode: InteractionMode) {
    setError(null);
    if (agent.availabilityState !== "available" && agent.availabilityState !== "unknown") {
      setError(agent.unavailableReason ?? t("agents.error.notAvailable", { agent: agent.displayName }));
      return;
    }
    if (!agent.supportedInteractionModes.includes(mode)) {
      setError(t("agents.error.supportedModes", { agent: agent.displayName, modes: agent.supportedInteractionModes.map((item) => t(`agents.mode.${item}`)).join(", ") }));
      return;
    }
    setSelectedMode(mode);
    await selectAgentMutation.mutateAsync({ agent, mode }).catch((err) => setError(err instanceof Error ? err.message : String(err)));
  }

  async function handleLaunch() {
    setError(null);
    setNotice(null);
    await launchWorkflowMutation.mutateAsync().catch((err) => setError(err instanceof Error ? err.message : String(err)));
  }

  function applyCapabilityFilter() {
    const next = capabilityFilter.trim();
    setError(null);
    if (next === appliedCapabilityFilter) {
      void agentsOverviewQuery.refetch();
    } else {
      setAppliedCapabilityFilter(next);
    }
  }

  return (
    <div className="space-y-4">
      <PageHeader
        actions={
          <Button disabled={agentsOverviewQuery.isFetching} variant="outline" onClick={() => void agentsOverviewQuery.refetch()}>
            <RefreshCw className="h-4 w-4" aria-hidden="true" />
            {agentsOverviewQuery.isFetching ? t("agents.refreshing") : t("agents.refresh")}
          </Button>
        }
        description={t("agents.description")}
        icon={Bot}
        title={t("agents.title")}
      />

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_340px]">
        <div className="space-y-4">
          <SectionPanel title={t("agents.filter.title")} description={t("agents.filter.description")}>
            <div className="flex flex-wrap gap-2">
              <input
                value={capabilityFilter}
                onChange={(event) => setCapabilityFilter(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") applyCapabilityFilter();
                }}
                className="ucd-input h-9 min-w-56 flex-1 rounded px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                placeholder={t("agents.filter.placeholder")}
              />
              <Button variant="outline" onClick={applyCapabilityFilter}>
                {t("agents.filter.apply")}
              </Button>
            </div>
          </SectionPanel>

          <div className="grid gap-4 lg:grid-cols-2">
            {filteredAgents.map((agent) => (
              <section className="ucd-panel ucd-interactive rounded-lg p-4" key={agent.id}>
                <div className="mb-4 flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <Terminal className="h-4 w-4 text-primary" aria-hidden="true" />
                      <h3 className="truncate font-semibold">{agent.displayName}</h3>
                    </div>
                    <p className="mt-1 text-sm text-muted-foreground">{agent.provider}</p>
                  </div>
                  <Badge tone={availabilityTone(agent)}>{agent.availabilityState}</Badge>
                </div>

                <TagList tags={agent.capabilityTags.slice(0, 3)} />

                <div className="mt-4 flex flex-wrap gap-2">
                  {agent.supportedInteractionModes.map((mode) => {
                    const Icon = modeIcons[mode];
                    return (
                      <button
                        className={`inline-flex h-8 items-center gap-1 rounded-md border px-2 text-xs ${
                          selectedMode === mode ? "border-primary bg-primary text-primary-foreground" : "border-border hover:bg-muted"
                        }`}
                        key={mode}
                        onClick={() => setSelectedMode(mode)}
                        title={t(`agents.mode.${mode}`)}
                        type="button"
                      >
                        <Icon className="h-3.5 w-3.5" aria-hidden="true" />
                        {t(`agents.mode.${mode}`)}
                      </button>
                    );
                  })}
                </div>

                <div className="mt-4 flex items-center justify-between gap-3">
                  <StatusPill status={workflow?.activeAgentId === agent.id ? t("agents.status.running") : t("agents.status.idle")} />
                  <Button variant="outline" onClick={() => void handleSelect(agent, selectedMode)}>
                    <CheckCircle2 className="h-4 w-4" aria-hidden="true" />
                    {t("agents.configure")}
                  </Button>
                </div>
              </section>
            ))}
          </div>
        </div>

        <SectionPanel title={t("agents.details.title")} description={t("agents.details.description")}>
          <dl className="grid gap-4 text-sm">
            <div>
              <dt className="text-xs uppercase text-muted-foreground">{t("agents.details.activeAgent")}</dt>
              <dd className="mt-1 font-medium">{activeAgent?.displayName ?? t("agents.details.noneSelected")}</dd>
            </div>
            <div>
              <dt className="text-xs uppercase text-muted-foreground">{t("agents.details.interactionMode")}</dt>
              <dd className="mt-1 font-medium">
                {workflow?.activeInteractionMode ? t(`agents.mode.${workflow.activeInteractionMode}`) : t("agents.details.notSelected")}
              </dd>
            </div>
            <div>
              <dt className="text-xs uppercase text-muted-foreground">{t("agents.details.lifecycle")}</dt>
              <dd className="mt-1 font-medium">{workflow?.lifecycleState ?? t("agents.status.idle")}</dd>
            </div>
            <div>
              <dt className="text-xs uppercase text-muted-foreground">{t("agents.details.intent")}</dt>
              <dd className="mt-1 text-muted-foreground">{workflow?.intent ?? t("agents.details.defaultIntent")}</dd>
            </div>
          </dl>

          {visibleError ? (
            <div className="mt-5 flex gap-2 rounded-md border p-3 text-sm ucd-status-warning">
              <CircleAlert className="mt-0.5 h-4 w-4 shrink-0" aria-hidden="true" />
              <span>{visibleError}</span>
            </div>
          ) : null}

          {notice ? <div className="mt-5 rounded-md border p-3 text-sm ucd-status-success">{notice}</div> : null}

          <Button className="mt-5 w-full" disabled={!activeAgent || launchWorkflowMutation.isPending} onClick={() => void handleLaunch()}>
            <Play className="h-4 w-4" aria-hidden="true" />
            {t("agents.launch")}
          </Button>

          <div className="mt-5 border-t border-border pt-4">
            <div className="mb-2 flex items-center gap-2 text-sm font-medium">
              <Activity className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
              {t("agents.details.session")}
            </div>
            {sessionDetails ? (
              <dl className="grid gap-2 text-xs text-muted-foreground">
                <div className="flex justify-between gap-3">
                  <dt>{t("agents.details.adapter")}</dt>
                  <dd className="font-medium text-foreground">{sessionDetails.adapter}</dd>
                </div>
                <div className="flex justify-between gap-3">
                  <dt>{t("agents.details.runtime")}</dt>
                  <dd className="font-medium text-foreground">{sessionDetails.details.runtime ?? "desktop"}</dd>
                </div>
              </dl>
            ) : null}
          </div>
        </SectionPanel>
      </div>
    </div>
  );
}
