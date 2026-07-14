import { useEffect, useMemo, useState } from "react";
import { Activity, CheckCircle2, CircleAlert, Laptop, Play, RefreshCw, Search, Terminal } from "lucide-react";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { agentService } from "../../services/runtime-agent-client";
import type { AgentRegistryEntry, InteractionMode, SessionDetails, WorkflowState } from "../../types/agent";
import { PageHeader, SectionPanel, StatusPill, TagList } from "./page-parts";

const modeLabels: Record<InteractionMode, string> = {
  browser: "Browser",
  "native-desktop": "Native",
  cli: "CLI",
};

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
  const [agents, setAgents] = useState<AgentRegistryEntry[]>([]);
  const [workflow, setWorkflow] = useState<WorkflowState | null>(null);
  const [capabilityFilter, setCapabilityFilter] = useState("");
  const [selectedMode, setSelectedMode] = useState<InteractionMode>("cli");
  const [sessionDetails, setSessionDetails] = useState<SessionDetails | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  async function refresh() {
    const [agentList, workflowState] = await Promise.all([
      agentService.listAgents(capabilityFilter.trim() || undefined),
      agentService.getWorkflowState(),
    ]);
    setAgents(agentList);
    setWorkflow(workflowState);
    setSessionDetails(await agentService.getSessionDetails());
    if (workflowState.activeInteractionMode) {
      setSelectedMode(workflowState.activeInteractionMode);
    } else if (agentList[0]) {
      setSelectedMode(defaultMode(agentList[0]));
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

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
      setError(agent.unavailableReason ?? `${agent.displayName} is not available.`);
      return;
    }
    if (!agent.supportedInteractionModes.includes(mode)) {
      setError(`${agent.displayName} supports ${agent.supportedInteractionModes.map((item) => modeLabels[item]).join(", ")}.`);
      return;
    }
    const next = await agentService.selectAgent(agent.id, mode);
    setWorkflow(next);
    setSelectedMode(mode);
    setNotice(`${agent.displayName} selected for ${modeLabels[mode]} mode.`);
    setSessionDetails(await agentService.getSessionDetails());
  }

  async function handleLaunch() {
    setError(null);
    setNotice(null);
    if (workflow?.activeAgentId && workflow.activeInteractionMode === "browser") {
      const readiness = await agentService.checkBrowserReadiness(workflow.activeAgentId);
      if (!readiness.ready) {
        setError(readiness.reason ?? "Browser mode is not ready.");
        return;
      }
    }
    const result = await agentService.launchActiveWorkflow();
    setWorkflow(result.workflow);
    setNotice(result.message);
    setSessionDetails(await agentService.getSessionDetails());
  }

  return (
    <div className="space-y-4">
      <PageHeader
        actions={
          <Button variant="outline" onClick={() => void refresh()}>
            <RefreshCw className="h-4 w-4" aria-hidden="true" />
            刷新
          </Button>
        }
        description="管理可用 AI Coding Agent、交互模式和当前工作流"
        title="Agents"
      />

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_340px]">
        <div className="space-y-4">
          <SectionPanel title="Agent 过滤" description="按 capability tag 过滤注册 Agent">
            <div className="flex flex-wrap gap-2">
              <input
                value={capabilityFilter}
                onChange={(event) => setCapabilityFilter(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") void refresh();
                }}
                className="ucd-input h-9 min-w-56 flex-1 rounded px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                placeholder="Filter capability tag"
              />
              <Button variant="outline" onClick={() => void refresh()}>
                Apply
              </Button>
            </div>
          </SectionPanel>

          <div className="grid gap-4 lg:grid-cols-2">
            {filteredAgents.map((agent) => (
              <section className="ucd-panel rounded-lg p-4" key={agent.id}>
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
                        title={modeLabels[mode]}
                        type="button"
                      >
                        <Icon className="h-3.5 w-3.5" aria-hidden="true" />
                        {modeLabels[mode]}
                      </button>
                    );
                  })}
                </div>

                <div className="mt-4 flex items-center justify-between gap-3">
                  <StatusPill status={workflow?.activeAgentId === agent.id ? "运行中" : "空闲"} />
                  <Button variant="outline" onClick={() => void handleSelect(agent, selectedMode)}>
                    <CheckCircle2 className="h-4 w-4" aria-hidden="true" />
                    配置
                  </Button>
                </div>
              </section>
            ))}
          </div>
        </div>

        <SectionPanel title="Agent 配置详情" description="当前工作流与会话状态">
          <dl className="grid gap-4 text-sm">
            <div>
              <dt className="text-xs uppercase text-muted-foreground">Active Agent</dt>
              <dd className="mt-1 font-medium">{activeAgent?.displayName ?? "None selected"}</dd>
            </div>
            <div>
              <dt className="text-xs uppercase text-muted-foreground">Interaction Mode</dt>
              <dd className="mt-1 font-medium">
                {workflow?.activeInteractionMode ? modeLabels[workflow.activeInteractionMode] : "Not selected"}
              </dd>
            </div>
            <div>
              <dt className="text-xs uppercase text-muted-foreground">Lifecycle</dt>
              <dd className="mt-1 font-medium">{workflow?.lifecycleState ?? "idle"}</dd>
            </div>
            <div>
              <dt className="text-xs uppercase text-muted-foreground">Intent</dt>
              <dd className="mt-1 text-muted-foreground">{workflow?.intent ?? "Current development workflow"}</dd>
            </div>
          </dl>

          {error ? (
            <div className="mt-5 flex gap-2 rounded-md border p-3 text-sm ucd-status-warning">
              <CircleAlert className="mt-0.5 h-4 w-4 shrink-0" aria-hidden="true" />
              <span>{error}</span>
            </div>
          ) : null}

          {notice ? <div className="mt-5 rounded-md border p-3 text-sm ucd-status-success">{notice}</div> : null}

          <Button className="mt-5 w-full" disabled={!activeAgent} onClick={() => void handleLaunch()}>
            <Play className="h-4 w-4" aria-hidden="true" />
            Launch
          </Button>

          <div className="mt-5 border-t border-border pt-4">
            <div className="mb-2 flex items-center gap-2 text-sm font-medium">
              <Activity className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
              Session Details
            </div>
            {sessionDetails ? (
              <dl className="grid gap-2 text-xs text-muted-foreground">
                <div className="flex justify-between gap-3">
                  <dt>Adapter</dt>
                  <dd className="font-medium text-foreground">{sessionDetails.adapter}</dd>
                </div>
                <div className="flex justify-between gap-3">
                  <dt>Runtime</dt>
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
