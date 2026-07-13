import { useEffect, useMemo, useState } from "react";
import {
  Activity,
  Bot,
  CheckCircle2,
  CircleAlert,
  Laptop,
  Play,
  RefreshCw,
  Search,
  Terminal,
} from "lucide-react";
import { Badge } from "./components/ui/badge";
import { Button } from "./components/ui/button";
import { agentService } from "./services/runtime-agent-client";
import type { AgentRegistryEntry, InteractionMode, SessionDetails, WorkflowState } from "./types/agent";

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

export function App() {
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
    <main className="min-h-screen bg-background text-foreground">
      <div className="grid min-h-screen grid-cols-[220px_1fr]">
        <aside className="border-r border-border bg-muted/40 px-4 py-5">
          <div className="flex items-center gap-2">
            <div className="flex h-9 w-9 items-center justify-center rounded-md bg-primary text-primary-foreground">
              <Bot className="h-5 w-5" aria-hidden="true" />
            </div>
            <div>
              <h1 className="text-base font-semibold">VaneHub AI</h1>
              <p className="text-xs text-muted-foreground">Agent control</p>
            </div>
          </div>

          <nav className="mt-8 grid gap-1 text-sm">
            <button className="rounded-md bg-background px-3 py-2 text-left font-medium shadow-sm">Agents</button>
            <button className="rounded-md px-3 py-2 text-left text-muted-foreground hover:bg-background">Workflows</button>
            <button className="rounded-md px-3 py-2 text-left text-muted-foreground hover:bg-background">Sessions</button>
            <button className="rounded-md px-3 py-2 text-left text-muted-foreground hover:bg-background">Settings</button>
          </nav>
        </aside>

        <section className="flex min-w-0 flex-col">
          <header className="flex h-16 items-center justify-between border-b border-border px-6">
            <div>
              <h2 className="text-lg font-semibold">Agent Switcher</h2>
              <p className="text-sm text-muted-foreground">Workspace: current repository</p>
            </div>
            <Button variant="outline" onClick={() => void refresh()}>
              <RefreshCw className="h-4 w-4" aria-hidden="true" />
              Refresh
            </Button>
          </header>

          <div className="grid flex-1 grid-cols-[minmax(0,1fr)_340px] gap-6 p-6">
            <div className="min-w-0">
              <div className="mb-4 flex items-center justify-between gap-4">
                <div className="relative w-full max-w-sm">
                  <Search className="pointer-events-none absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
                  <input
                    value={capabilityFilter}
                    onChange={(event) => setCapabilityFilter(event.target.value)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") void refresh();
                    }}
                    className="h-9 w-full rounded-md border border-input bg-background pl-9 pr-3 text-sm outline-none ring-offset-background focus-visible:ring-2 focus-visible:ring-ring"
                    placeholder="Filter capability tag"
                  />
                </div>
                <Button variant="outline" onClick={() => void refresh()}>
                  Apply
                </Button>
              </div>

              <div className="overflow-hidden rounded-lg border border-border">
                <div className="grid grid-cols-[1.5fr_1fr_1fr_1.3fr_110px] border-b border-border bg-muted/60 px-4 py-2 text-xs font-medium uppercase text-muted-foreground">
                  <span>Agent</span>
                  <span>Provider</span>
                  <span>Status</span>
                  <span>Modes</span>
                  <span className="text-right">Action</span>
                </div>
                {agents.map((agent) => (
                  <div
                    className="grid min-h-20 grid-cols-[1.5fr_1fr_1fr_1.3fr_110px] items-center border-b border-border px-4 py-3 last:border-b-0"
                    key={agent.id}
                  >
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <Terminal className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
                        <span className="truncate font-medium">{agent.displayName}</span>
                      </div>
                      <div className="mt-1 flex flex-wrap gap-1">
                        {agent.capabilityTags.slice(0, 3).map((tag) => (
                          <Badge key={tag} tone="muted">
                            {tag}
                          </Badge>
                        ))}
                      </div>
                    </div>
                    <span className="text-sm text-muted-foreground">{agent.provider}</span>
                    <Badge tone={availabilityTone(agent)}>{agent.availabilityState}</Badge>
                    <div className="flex flex-wrap gap-1">
                      {agent.supportedInteractionModes.map((mode) => {
                        const Icon = modeIcons[mode];
                        return (
                          <button
                            className={`inline-flex h-8 items-center gap-1 rounded-md border px-2 text-xs ${
                              selectedMode === mode ? "border-primary bg-primary text-primary-foreground" : "border-border"
                            }`}
                            key={mode}
                            onClick={() => setSelectedMode(mode)}
                            title={modeLabels[mode]}
                          >
                            <Icon className="h-3.5 w-3.5" aria-hidden="true" />
                            {modeLabels[mode]}
                          </button>
                        );
                      })}
                    </div>
                    <div className="text-right">
                      <Button size="icon" variant="outline" onClick={() => void handleSelect(agent, selectedMode)}>
                        <CheckCircle2 className="h-4 w-4" aria-hidden="true" />
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            <aside className="rounded-lg border border-border bg-background p-4">
              <div className="mb-4 flex items-center justify-between">
                <h3 className="text-sm font-semibold">Current Workflow</h3>
                <Badge tone="muted">{workflow?.lifecycleState ?? "idle"}</Badge>
              </div>

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
                  <dt className="text-xs uppercase text-muted-foreground">Intent</dt>
                  <dd className="mt-1 text-muted-foreground">{workflow?.intent ?? "Current development workflow"}</dd>
                </div>
              </dl>

              {error ? (
                <div className="mt-5 flex gap-2 rounded-md border border-amber-200 bg-amber-50 p-3 text-sm text-amber-800">
                  <CircleAlert className="mt-0.5 h-4 w-4 shrink-0" aria-hidden="true" />
                  <span>{error}</span>
                </div>
              ) : null}

              {notice ? (
                <div className="mt-5 rounded-md border border-emerald-200 bg-emerald-50 p-3 text-sm text-emerald-800">
                  {notice}
                </div>
              ) : null}

              <Button className="mt-5 w-full" disabled={!activeAgent} onClick={() => void handleLaunch()}>
                <Play className="h-4 w-4" aria-hidden="true" />
                Launch
              </Button>

              <div className="mt-5 border-t border-border pt-4">
                <div className="mb-2 flex items-center gap-2 text-sm font-medium">
                  <Activity className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
                  Session Details
                </div>
                <p className="text-sm text-muted-foreground">
                  Agent-specific session metadata stays inside its adapter while this panel shows the common lifecycle.
                </p>
                {sessionDetails ? (
                  <dl className="mt-3 grid gap-2 text-xs text-muted-foreground">
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
            </aside>
          </div>
        </section>
      </div>
    </main>
  );
}
