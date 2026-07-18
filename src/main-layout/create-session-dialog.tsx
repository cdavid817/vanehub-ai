import { useEffect, useMemo, useState } from "react";
import { Loader2, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { agentService } from "../services/runtime-agent-client";
import { operationService } from "../services/runtime-operation-client";
import { CreateSessionAgentSection } from "./create-session-agent-section";
import {
  LocalWorkspaceSection,
  RemoteWorkspaceSection,
  WorkspaceModeSelector,
  type WorkspaceMode,
} from "./create-session-workspace-sections";
import type {
  AgentRegistryEntry,
  CreateSessionInput,
  InteractionMode,
  KnownRemoteWorkspace,
  KnownProject,
  ProjectInspection,
  Session,
} from "../types/agent";
import type { OperationTask } from "../types/operation";

const preferredAgentIds = ["claude-code", "gemini-cli", "codex-cli", "opencode"];

function firstMode(agent: AgentRegistryEntry | null): InteractionMode { return agent?.supportedInteractionModes[0] ?? "cli"; }

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
}: { agents: AgentRegistryEntry[]; onClose: () => void; onCreated: (session: Session) => void; open: boolean }) {
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
  const [workspaceMode, setWorkspaceMode] = useState<WorkspaceMode>("local");
  const [projectPath, setProjectPath] = useState("");
  const [knownProjects, setKnownProjects] = useState<KnownProject[]>([]);
  const [knownRemoteWorkspaces, setKnownRemoteWorkspaces] = useState<KnownRemoteWorkspace[]>([]);
  const [inspection, setInspection] = useState<ProjectInspection | null>(null);
  const [worktreeEnabled, setWorktreeEnabled] = useState(false);
  const [worktreeName, setWorktreeName] = useState("");
  const [remoteHost, setRemoteHost] = useState("");
  const [remoteUser, setRemoteUser] = useState("");
  const [remotePath, setRemotePath] = useState("");
  const [remoteDisplayName, setRemoteDisplayName] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [createOperationId, setCreateOperationId] = useState<string | null>(null);
  const [handledCreateOperationId, setHandledCreateOperationId] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    const agent = availableAgents[0] ?? null;
    setAgentId(agent?.id ?? "");
    setInteractionMode(firstMode(agent));
    setWorkspaceMode("local");
    setError(null);
    void agentService.listKnownProjects().then(setKnownProjects).catch(() => setKnownProjects([]));
    void agentService.listKnownRemoteWorkspaces().then(setKnownRemoteWorkspaces).catch(() => setKnownRemoteWorkspaces([]));
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
    if (!selectedAgent) return;
    if (workspaceMode === "local" && !projectPath.trim()) return;
    if (workspaceMode === "remote" && (!remoteHost.trim() || !remotePath.trim())) return;
    setLoading(true);
    setError(null);
    const input: CreateSessionInput = {
      agentId: selectedAgent.id,
      interactionMode,
      title,
      projectPath: workspaceMode === "local" ? projectPath : null,
      folder: workspaceMode === "local" ? projectPath : null,
      remoteWorkspace:
        workspaceMode === "remote"
          ? {
              host: remoteHost,
              user: remoteUser || null,
              path: remotePath,
              displayName: remoteDisplayName || null,
            }
          : null,
      worktree: workspaceMode === "local" && worktreeEnabled ? { enabled: true, name: worktreeName } : null,
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
  const canSubmit = Boolean(
    selectedAgent &&
      (workspaceMode === "remote"
        ? remoteHost.trim() && remotePath.trim()
        : projectPath.trim() && (!worktreeEnabled || worktreeName.trim())),
  );

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
            <CreateSessionAgentSection
              agents={availableAgents}
              interactionMode={interactionMode}
              onAgentSelect={(agent) => {
                setAgentId(agent.id);
                setInteractionMode(firstMode(agent));
              }}
              onInteractionModeChange={setInteractionMode}
              selectedAgent={selectedAgent}
            />

            <WorkspaceModeSelector
              mode={workspaceMode}
              onModeChange={(mode) => {
                setWorkspaceMode(mode);
                setWorktreeEnabled(false);
                setError(null);
              }}
            />

            {workspaceMode === "local" ? (
              <LocalWorkspaceSection
                gitCapable={gitCapable}
                inspection={inspection}
                knownProjects={knownProjects}
                onBrowseProject={() => void browseProject()}
                onInspectPath={(path) => void inspectPath(path)}
                projectPath={projectPath}
                setProjectPath={setProjectPath}
                setWorktreeEnabled={setWorktreeEnabled}
                setWorktreeName={setWorktreeName}
                worktreeEnabled={worktreeEnabled}
                worktreeName={worktreeName}
              />
            ) : (
              <RemoteWorkspaceSection
                knownRemoteWorkspaces={knownRemoteWorkspaces}
                remoteDisplayName={remoteDisplayName}
                remoteHost={remoteHost}
                remotePath={remotePath}
                remoteUser={remoteUser}
                setRemoteDisplayName={setRemoteDisplayName}
                setRemoteHost={setRemoteHost}
                setRemotePath={setRemotePath}
                setRemoteUser={setRemoteUser}
              />
            )}

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
