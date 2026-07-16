import { useEffect, useMemo, useState } from "react";
import { Bot, Folder, GitBranch, Loader2, X } from "lucide-react";
import { Button } from "../components/ui/button";
import { cn } from "../lib/utils";
import { agentService } from "../services/runtime-agent-client";
import type {
  AgentRegistryEntry,
  CreateSessionInput,
  InteractionMode,
  KnownProject,
  ProjectInspection,
  Session,
} from "../types/agent";

const preferredAgentIds = ["claude-code", "gemini-cli", "codex-cli", "opencode"];

function firstMode(agent: AgentRegistryEntry | null): InteractionMode {
  return agent?.supportedInteractionModes[0] ?? "cli";
}

function conciseError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  if (message.includes("Git")) return "Git worktree failed";
  if (message.includes("Project")) return "Project unavailable";
  if (message.includes("Agent")) return "Agent unavailable";
  return "Command failed";
}

export function CreateSessionDialog({
  agents,
  onClose,
  onCreated,
  open,
}: {
  agents: AgentRegistryEntry[];
  onClose: () => void;
  onCreated: (session: Session) => void;
  open: boolean;
}) {
  const availableAgents = useMemo(
    () =>
      preferredAgentIds
        .map((agentId) => agents.find((agent) => agent.id === agentId))
        .filter((agent): agent is AgentRegistryEntry => Boolean(agent)),
    [agents],
  );
  const [agentId, setAgentId] = useState("");
  const selectedAgent = availableAgents.find((agent) => agent.id === agentId) ?? availableAgents[0] ?? null;
  const [interactionMode, setInteractionMode] = useState<InteractionMode>("cli");
  const [title, setTitle] = useState("");
  const [projectPath, setProjectPath] = useState("");
  const [knownProjects, setKnownProjects] = useState<KnownProject[]>([]);
  const [inspection, setInspection] = useState<ProjectInspection | null>(null);
  const [worktreeEnabled, setWorktreeEnabled] = useState(false);
  const [worktreeName, setWorktreeName] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    const agent = availableAgents[0] ?? null;
    setAgentId(agent?.id ?? "");
    setInteractionMode(firstMode(agent));
    setError(null);
    void agentService.listKnownProjects().then(setKnownProjects).catch(() => setKnownProjects([]));
  }, [availableAgents, open]);

  useEffect(() => {
    if (!selectedAgent) return;
    if (!selectedAgent.supportedInteractionModes.includes(interactionMode)) {
      setInteractionMode(firstMode(selectedAgent));
    }
  }, [interactionMode, selectedAgent]);

  async function inspectPath(path: string) {
    const trimmed = path.trim();
    setProjectPath(trimmed);
    setWorktreeEnabled(false);
    setWorktreeName("");
    setInspection(null);
    setError(null);
    if (!trimmed) return;
    try {
      setInspection(await agentService.inspectProject(trimmed));
    } catch (inspectionError) {
      setError(conciseError(inspectionError));
    }
  }

  async function browseProject() {
    setError(null);
    try {
      const selectedPath = await agentService.selectProjectDirectory();
      if (selectedPath) {
        await inspectPath(selectedPath);
      }
    } catch (browseError) {
      setError(conciseError(browseError));
    }
  }

  async function submit() {
    if (!selectedAgent || !projectPath.trim()) return;
    setLoading(true);
    setError(null);
    const input: CreateSessionInput = {
      agentId: selectedAgent.id,
      interactionMode,
      title,
      projectPath,
      folder: projectPath,
      worktree: worktreeEnabled ? { enabled: true, name: worktreeName } : null,
    };
    try {
      const session = await agentService.createSession(input);
      onCreated(session);
    } catch (createError) {
      setError(conciseError(createError));
    } finally {
      setLoading(false);
    }
  }

  if (!open) return null;
  const gitCapable = inspection?.isGit ?? false;
  const canSubmit = Boolean(selectedAgent && projectPath.trim() && (!worktreeEnabled || worktreeName.trim()));

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-background/70 p-4">
      <div className="grid max-h-[88vh] w-full max-w-2xl grid-rows-[auto_minmax(0,1fr)_auto] overflow-hidden rounded-lg border border-border bg-background shadow-xl">
        <div className="flex items-center justify-between border-b border-border p-4">
          <div>
            <h3 className="text-sm font-semibold">创建会话</h3>
            <p className="mt-1 text-xs text-muted-foreground">选择 Agent、项目文件夹和可选 Git worktree。</p>
          </div>
          <Button className="h-8 w-8 px-0" onClick={onClose} variant="outline">
            <X className="h-4 w-4" aria-hidden="true" />
          </Button>
        </div>

        <div className="min-h-0 overflow-y-auto p-4">
          <div className="grid gap-4">
            <section className="grid gap-2">
              <span className="text-xs font-medium text-muted-foreground">Agent</span>
              <div className="grid grid-cols-2 gap-2">
                {availableAgents.map((agent) => (
                  <button
                    className={cn(
                      "flex min-h-12 items-center gap-2 rounded border border-border p-2 text-left text-sm hover:bg-muted",
                      selectedAgent?.id === agent.id && "border-primary bg-[hsl(var(--nav-active-soft))]",
                    )}
                    key={agent.id}
                    onClick={() => {
                      setAgentId(agent.id);
                      setInteractionMode(firstMode(agent));
                    }}
                    type="button"
                  >
                    <Bot className="h-4 w-4 text-primary" aria-hidden="true" />
                    <span className="min-w-0">
                      <span className="block truncate font-medium">{agent.displayName}</span>
                      <span className="block truncate text-xs text-muted-foreground">{agent.id}</span>
                    </span>
                  </button>
                ))}
              </div>
              <div className="flex flex-wrap gap-2">
                {selectedAgent?.supportedInteractionModes.map((mode) => (
                  <button
                    className={cn(
                      "h-7 rounded border border-border px-2 text-xs hover:bg-muted",
                      interactionMode === mode && "border-primary bg-primary text-primary-foreground",
                    )}
                    key={mode}
                    onClick={() => setInteractionMode(mode)}
                    type="button"
                  >
                    {mode}
                  </button>
                ))}
              </div>
            </section>

            <section className="grid gap-2">
              <span className="text-xs font-medium text-muted-foreground">项目文件夹</span>
              <div className="flex gap-2">
                <input
                  className="ucd-input h-9 min-w-0 flex-1 rounded px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  onBlur={() => void inspectPath(projectPath)}
                  onChange={(event) => setProjectPath(event.target.value)}
                  placeholder="D:\\code\\project"
                  value={projectPath}
                />
                <Button className="h-9 px-3 text-xs" onClick={browseProject} type="button" variant="outline">
                  <Folder className="h-3.5 w-3.5" aria-hidden="true" />
                  浏览
                </Button>
              </div>
              {knownProjects.length > 0 ? (
                <div className="grid gap-1">
                  {knownProjects.slice(0, 5).map((project) => (
                    <button
                      className="flex items-center gap-2 rounded border border-border px-2 py-1.5 text-left text-xs hover:bg-muted"
                      key={project.path}
                      onClick={() => void inspectPath(project.path)}
                      type="button"
                    >
                      <Folder className="h-3.5 w-3.5 text-primary" aria-hidden="true" />
                      <span className="min-w-0 flex-1 truncate">{project.path}</span>
                      <span className="text-muted-foreground">{project.isGit ? "Git" : "Folder"}</span>
                    </button>
                  ))}
                </div>
              ) : null}
              {inspection ? (
                <p className="text-xs text-muted-foreground">
                  {inspection.isGit ? "Git 项目，可创建 worktree。" : "普通文件夹，将禁用 worktree。"}
                </p>
              ) : null}
            </section>

            <section className="grid gap-2 rounded border border-border p-3">
              <label className={cn("flex items-center gap-2 text-sm", !gitCapable && "text-muted-foreground")}>
                <input
                  checked={worktreeEnabled}
                  className="h-4 w-4"
                  disabled={!gitCapable}
                  onChange={(event) => setWorktreeEnabled(event.target.checked)}
                  type="checkbox"
                />
                <GitBranch className="h-4 w-4" aria-hidden="true" />
                创建新 Git worktree
              </label>
              {worktreeEnabled ? (
                <label className="grid gap-1">
                  <span className="text-xs text-muted-foreground">Worktree 名称</span>
                  <input
                    className="ucd-input h-9 rounded px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    onChange={(event) => setWorktreeName(event.target.value)}
                    placeholder="feature-a"
                    value={worktreeName}
                  />
                  <span className="text-xs text-muted-foreground">默认路径：项目同级目录 + 项目名-worktreeName；分支：vanehub/worktreeName</span>
                </label>
              ) : null}
            </section>

            <label className="grid gap-1">
              <span className="text-xs font-medium text-muted-foreground">会话名称</span>
              <input
                className="ucd-input h-9 rounded px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                onChange={(event) => setTitle(event.target.value)}
                placeholder="新会话"
                value={title}
              />
            </label>
          </div>
        </div>

        <div className="flex items-center justify-between gap-3 border-t border-border p-4">
          <span className="min-w-0 truncate text-xs text-destructive">{error}</span>
          <div className="flex gap-2">
            <Button className="h-8 px-3 text-xs" onClick={onClose} type="button" variant="outline">取消</Button>
            <Button className="h-8 px-3 text-xs" disabled={!canSubmit || loading} onClick={submit} type="button">
              {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" aria-hidden="true" /> : null}
              创建
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
