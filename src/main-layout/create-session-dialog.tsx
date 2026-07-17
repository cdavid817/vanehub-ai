import { useEffect, useMemo, useState } from "react";
import { Bot, Folder, GitBranch, Loader2, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { cn } from "../lib/utils";
import { agentService } from "../services/runtime-agent-client";
import { operationService } from "../services/runtime-operation-client";
import type {
  AgentRegistryEntry,
  CreateSessionInput,
  InteractionMode,
  KnownProject,
  ProjectInspection,
  Session,
} from "../types/agent";
import type { OperationTask } from "../types/operation";

const preferredAgentIds = ["claude-code", "gemini-cli", "codex-cli", "opencode"];

function firstMode(agent: AgentRegistryEntry | null): InteractionMode {
  return agent?.supportedInteractionModes[0] ?? "cli";
}

function conciseError(error: unknown, t: (key: string) => string) {
  const message = error instanceof Error ? error.message : String(error);
  if (message.includes("Git")) return t("createSession.error.git");
  if (message.includes("Project")) return t("createSession.error.project");
  if (message.includes("Agent")) return t("createSession.error.agent");
  return t("createSession.error.command");
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
  const { t } = useTranslation();
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
  const [createOperationId, setCreateOperationId] = useState<string | null>(null);
  const [handledCreateOperationId, setHandledCreateOperationId] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    const agent = availableAgents[0] ?? null;
    setAgentId(agent?.id ?? "");
    setInteractionMode(firstMode(agent));
    setError(null);
    void agentService.listKnownProjects().then(setKnownProjects).catch(() => setKnownProjects([]));
  }, [availableAgents, open]);

  useEffect(() => {
    if (!createOperationId || handledCreateOperationId === createOperationId) return;
    const operationId = createOperationId;
    let cancelled = false;
    let timer: number | undefined;

    async function pollOperation() {
      try {
        const operation = await operationService.getOperationStatus(operationId);
        if (cancelled) return;
        if (operation.status === "queued" || operation.status === "running") {
          timer = window.setTimeout(() => void pollOperation(), 600);
          return;
        }
        setHandledCreateOperationId(operation.id);
        setLoading(false);
        if (operation.status === "failed") {
          setError(operation.error ?? t("createSession.error.command"));
          return;
        }
        const session = sessionResult(operation.result);
        if (!session) {
          setError(t("createSession.error.command"));
          return;
        }
        onCreated(session);
      } catch (operationError) {
        if (!cancelled) {
          setLoading(false);
          setError(conciseError(operationError, t));
        }
      }
    }

    void pollOperation();
    return () => {
      cancelled = true;
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, [createOperationId, handledCreateOperationId, onCreated, t]);

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
      setError(conciseError(inspectionError, t));
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
      setError(conciseError(browseError, t));
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
      const operation = await agentService.createSession(input);
      setCreateOperationId(operation.id);
      setHandledCreateOperationId(null);
    } catch (createError) {
      setError(conciseError(createError, t));
      setLoading(false);
    }
  }

  if (!open) return null;
  const gitCapable = inspection?.isGit ?? false;
  const canSubmit = Boolean(selectedAgent && projectPath.trim() && (!worktreeEnabled || worktreeName.trim()));

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-background/70 p-4">
      <div className="ucd-panel grid max-h-[88vh] w-full max-w-2xl grid-rows-[auto_minmax(0,1fr)_auto] overflow-hidden rounded-lg shadow-xl">
        <div className="flex items-center justify-between border-b border-border p-4">
          <div>
            <h3 className="text-sm font-semibold">{t("createSession.title")}</h3>
            <p className="mt-1 text-xs text-muted-foreground">{t("createSession.description")}</p>
          </div>
          <Button className="h-8 w-8 px-0" onClick={onClose} variant="outline">
            <X className="h-4 w-4" aria-hidden="true" />
          </Button>
        </div>

        <div className="min-h-0 overflow-y-auto p-4">
          <div className="grid gap-4">
            <section className="grid gap-2">
              <span className="text-xs font-medium text-muted-foreground">{t("createSession.agent")}</span>
              <div className="grid grid-cols-2 gap-2">
                {availableAgents.map((agent) => (
                  <button
                    className={cn(
                      "ucd-list-row flex min-h-12 items-center gap-2 rounded-md p-2 text-left text-sm",
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
                      "h-7 rounded-md border border-border px-2 text-xs hover:bg-muted",
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
              <span className="text-xs font-medium text-muted-foreground">{t("createSession.projectFolder")}</span>
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
                  {t("createSession.browse")}
                </Button>
              </div>
              {knownProjects.length > 0 ? (
                <div className="grid gap-1">
                  {knownProjects.slice(0, 5).map((project) => (
                    <button
                      className="ucd-list-row flex items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs"
                      key={project.path}
                      onClick={() => void inspectPath(project.path)}
                      type="button"
                    >
                      <Folder className="h-3.5 w-3.5 text-primary" aria-hidden="true" />
                      <span className="min-w-0 flex-1 truncate">{project.path}</span>
                      <span className="text-muted-foreground">{project.isGit ? t("createSession.folderType.git") : t("createSession.folderType.folder")}</span>
                    </button>
                  ))}
                </div>
              ) : null}
              {inspection ? (
                <p className="text-xs text-muted-foreground">
                  {inspection.isGit ? t("createSession.gitProject") : t("createSession.normalFolder")}
                </p>
              ) : null}
            </section>

            <section className="ucd-muted-panel grid gap-2 rounded-md p-3">
              <label className={cn("flex items-center gap-2 text-sm", !gitCapable && "text-muted-foreground")}>
                <input
                  checked={worktreeEnabled}
                  className="h-4 w-4"
                  disabled={!gitCapable}
                  onChange={(event) => setWorktreeEnabled(event.target.checked)}
                  type="checkbox"
                />
                <GitBranch className="h-4 w-4" aria-hidden="true" />
                {t("createSession.createWorktree")}
              </label>
              {worktreeEnabled ? (
                <label className="grid gap-1">
                  <span className="text-xs text-muted-foreground">{t("createSession.worktreeName")}</span>
                  <input
                    className="ucd-input h-9 rounded px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    onChange={(event) => setWorktreeName(event.target.value)}
                    placeholder="feature-a"
                    value={worktreeName}
                  />
                  <span className="text-xs text-muted-foreground">{t("createSession.worktreeHint")}</span>
                </label>
              ) : null}
            </section>

            <label className="grid gap-1">
              <span className="text-xs font-medium text-muted-foreground">{t("createSession.sessionName")}</span>
              <input
                className="ucd-input h-9 rounded px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                onChange={(event) => setTitle(event.target.value)}
                placeholder={t("createSession.sessionPlaceholder")}
                value={title}
              />
            </label>
          </div>
        </div>

        <div className="flex items-center justify-between gap-3 border-t border-border p-4">
          <span className="min-w-0 truncate text-xs text-destructive">{error}</span>
          <div className="flex gap-2">
            <Button className="h-8 px-3 text-xs" onClick={onClose} type="button" variant="outline">{t("createSession.cancel")}</Button>
            <Button className="h-8 px-3 text-xs" disabled={!canSubmit || loading} onClick={submit} type="button">
              {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" aria-hidden="true" /> : null}
              {t("createSession.create")}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}

function sessionResult(result: OperationTask["result"]): Session | null {
  if (!result || typeof result !== "object") return null;
  if (typeof result.id !== "string") return null;
  if (typeof result.agentId !== "string") return null;
  if (typeof result.interactionMode !== "string") return null;
  return result as unknown as Session;
}
