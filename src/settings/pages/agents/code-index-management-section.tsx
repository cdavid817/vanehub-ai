import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Database, FolderPlus, LoaderCircle, ShieldAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type { CodeIndexConfigurationInput, CodeIndexWorkspace } from "../../../types/code-index";
import { CodeEmbeddingConfirmationDialog, CodeIndexDestructiveDialog } from "./code-index-confirmation-dialogs";
import { CodeIndexConfigurationDialog } from "./code-index-configuration-dialog";
import { CodeIndexWorkspaceRow } from "./code-index-workspace-row";

const workspaceKey = ["agents", "workspace-code-indexes"] as const;
const retrievalKey = ["agents", "onepiece-retrieval-configuration"] as const;

type PendingDialog = { workspace: CodeIndexWorkspace; action: "rebuild" | "delete" } | null;

function displayNameFromPath(path: string): string {
  return path.replace(/[\\/]+$/, "").split(/[\\/]/).pop() || path;
}

export function CodeIndexManagementSection({ service = defaultAgentService }: { service?: AgentService }) {
  const { t } = useTranslation();
  const [editing, setEditing] = useState<CodeIndexWorkspace | null>(null);
  const [confirmingEmbedding, setConfirmingEmbedding] = useState<CodeIndexWorkspace | null>(null);
  const [pendingDialog, setPendingDialog] = useState<PendingDialog>(null);
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const workspacesQuery = useQuery({
    queryKey: workspaceKey,
    queryFn: () => service.listCodeIndexWorkspaces(),
    refetchInterval: 3_000,
  });
  const retrievalQuery = useQuery({ queryKey: retrievalKey, queryFn: () => service.getRetrievalConfiguration() });
  const workspaces = workspacesQuery.data ?? [];

  async function run(key: string, operation: () => Promise<unknown>) {
    setBusyKey(key);
    setError(null);
    try {
      await operation();
      await workspacesQuery.refetch();
      return true;
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
      return false;
    } finally {
      setBusyKey(null);
    }
  }

  async function addWorkspace() {
    const root = await service.selectProjectDirectory();
    if (!root) return;
    await run("add", () => service.registerCodeIndexWorkspace(root, displayNameFromPath(root)));
  }

  async function saveConfiguration(workspace: CodeIndexWorkspace, configuration: CodeIndexConfigurationInput) {
    if (await run(workspace.workspaceId, () => service.saveCodeIndexConfiguration(workspace.workspaceId, configuration))) {
      setEditing(null);
    }
  }

  async function confirmEmbedding(workspace: CodeIndexWorkspace) {
    const configuration = retrievalQuery.data;
    if (!configuration?.sourceProfileId || !configuration.embeddingModel) return;
    const profileId = configuration.sourceProfileId;
    const model = configuration.embeddingModel;
    const confirmed = await run(workspace.workspaceId, () => service.confirmCodeIndexEmbedding(
      workspace.workspaceId,
      profileId,
      model,
      workspace.generation,
    ));
    if (confirmed) setConfirmingEmbedding(null);
  }

  async function confirmDestructive() {
    if (!pendingDialog) return;
    const { workspace, action } = pendingDialog;
    const completed = await run(workspace.workspaceId, () => action === "rebuild"
      ? service.rebuildCodeIndexWorkspace(workspace.workspaceId)
      : service.deleteCodeIndexWorkspace(workspace.workspaceId));
    if (completed) setPendingDialog(null);
  }

  if (workspacesQuery.isLoading) {
    return <div className="flex min-h-24 items-center justify-center gap-2 text-sm text-muted-foreground"><LoaderCircle className="h-4 w-4 animate-spin" />{t("agents.globalConfig.loading")}</div>;
  }

  const configuration = retrievalQuery.data;
  const canConfirmEmbedding = Boolean(configuration?.sourceProfileId && configuration.embeddingModel);
  const totals = workspaces.reduce((current, workspace) => ({
    files: current.files + workspace.status.totalFiles,
    chunks: current.chunks + workspace.status.totalChunks,
    pending: current.pending + workspace.status.pendingChunks,
    enabled: current.enabled + Number(workspace.enabled),
  }), { files: 0, chunks: 0, pending: 0, enabled: 0 });
  return (
    <section aria-labelledby="code-index-heading" className="overflow-hidden rounded-lg border border-border bg-background">
      <div className="flex flex-col gap-3 border-b border-border bg-muted/20 px-4 py-4 sm:flex-row sm:items-start sm:justify-between sm:px-5">
        <div className="flex min-w-0 gap-3">
          <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-border bg-background text-primary"><Database className="h-4 w-4" /></span>
          <div><h3 className="text-sm font-semibold" id="code-index-heading">{t("codeIndex.title")}</h3><p className="mt-1 max-w-3xl text-xs leading-5 text-muted-foreground">{t("codeIndex.description")}</p></div>
        </div>
        <Button disabled={busyKey === "add"} onClick={() => void addWorkspace()} size="sm"><FolderPlus />{t("codeIndex.add")}</Button>
      </div>

      <div className="flex items-start gap-2 border-b border-border bg-muted/10 px-4 py-3 text-xs leading-5 text-muted-foreground sm:px-5">
        <ShieldAlert className="mt-0.5 h-4 w-4 shrink-0 text-primary" />
        <p>{t("codeIndex.privacy")}</p>
      </div>

      {workspaces.length ? (
        <dl className="grid grid-cols-2 border-b border-border bg-background text-xs sm:grid-cols-4">
          <div className="border-b border-border px-4 py-3 sm:border-b-0 sm:border-r"><dt className="text-muted-foreground">{t("codeIndex.aggregate.enabled")}</dt><dd className="mt-1 font-semibold tabular-nums">{totals.enabled}/{workspaces.length}</dd></div>
          <div className="border-b border-border px-4 py-3 sm:border-b-0 sm:border-r"><dt className="text-muted-foreground">{t("codeIndex.aggregate.files")}</dt><dd className="mt-1 font-semibold tabular-nums">{totals.files}</dd></div>
          <div className="border-r border-border px-4 py-3"><dt className="text-muted-foreground">{t("codeIndex.aggregate.chunks")}</dt><dd className="mt-1 font-semibold tabular-nums">{totals.chunks}</dd></div>
          <div className="px-4 py-3"><dt className="text-muted-foreground">{t("codeIndex.aggregate.pending")}</dt><dd className="mt-1 font-semibold tabular-nums">{totals.pending}</dd></div>
        </dl>
      ) : null}

      {workspaces.length === 0 ? <p className="px-5 py-8 text-center text-sm text-muted-foreground">{t("codeIndex.empty")}</p> : workspaces.map((workspace) => (
        <CodeIndexWorkspaceRow
          busy={busyKey === workspace.workspaceId}
          key={workspace.workspaceId}
          onConfigure={() => setEditing(workspace)}
          onConfirmEmbedding={() => canConfirmEmbedding ? setConfirmingEmbedding(workspace) : setError(t("codeIndex.confirmation.noModel"))}
          onDelete={() => setPendingDialog({ workspace, action: "delete" })}
          onDisable={() => void run(workspace.workspaceId, () => service.disableCodeIndexWorkspace(workspace.workspaceId))}
          onRebuild={() => setPendingDialog({ workspace, action: "rebuild" })}
          onRefresh={() => void run(workspace.workspaceId, () => service.refreshCodeIndexWorkspace(workspace.workspaceId))}
          workspace={workspace}
        />
      ))}

      {error ?? workspacesQuery.error ? <p className="border-t border-border px-4 py-3 text-sm ucd-status-warning" role="alert">{error ?? (workspacesQuery.error instanceof Error ? workspacesQuery.error.message : String(workspacesQuery.error))}</p> : null}
      {editing ? <CodeIndexConfigurationDialog pending={busyKey === editing.workspaceId} workspace={editing} onClose={() => setEditing(null)} onSave={(value) => saveConfiguration(editing, value)} /> : null}
      {confirmingEmbedding && configuration?.sourceProfileId && configuration.embeddingModel ? <CodeEmbeddingConfirmationDialog model={configuration.embeddingModel} onClose={() => setConfirmingEmbedding(null)} onConfirm={() => void confirmEmbedding(confirmingEmbedding)} pending={busyKey === confirmingEmbedding.workspaceId} profileId={configuration.sourceProfileId} workspace={confirmingEmbedding} /> : null}
      {pendingDialog ? <CodeIndexDestructiveDialog action={pendingDialog.action} onClose={() => setPendingDialog(null)} onConfirm={() => void confirmDestructive()} pending={busyKey === pendingDialog.workspace.workspaceId} workspace={pendingDialog.workspace} /> : null}
    </section>
  );
}
