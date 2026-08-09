import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Ban, LoaderCircle, RefreshCw, RotateCcw, Settings2, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { formatAppDateTime } from "../i18n/format";
import { resolveCodeIndexChannelStatus } from "../services/code-index-contract";
import type { AgentService } from "../services/agent-service";
import { agentService as defaultAgentService } from "../services/runtime-agent-client";
import { CodeEmbeddingConfirmationDialog, CodeIndexDestructiveDialog } from "../settings/pages/agents/code-index-confirmation-dialogs";
import { CodeIndexConfigurationDialog } from "../settings/pages/agents/code-index-configuration-dialog";
import type { CodeIndexConfigurationInput, CodeIndexWorkspace } from "../types/code-index";

type PendingDialog = { workspace: CodeIndexWorkspace; action: "rebuild" | "delete" } | null;

function pathKey(path: string): string {
  return path.replaceAll("\\", "/").replace(/^\/\/\?\//, "").replace(/\/$/, "").toLocaleLowerCase();
}

function Progress({ label, processed, total }: { label: string; processed: number; total: number }) {
  return <div><div className="mb-1 flex justify-between gap-2 text-[11px]"><span className="text-muted-foreground">{label}</span><span className="tabular-nums">{processed}/{total}</span></div><progress aria-label={label} className="h-1.5 w-full accent-primary" max={Math.max(total, 1)} value={processed} /></div>;
}

export function SessionCodeIndexPane({ workspacePath, service = defaultAgentService }: { workspacePath: string; service?: AgentService }) {
  const { i18n, t } = useTranslation();
  const [editing, setEditing] = useState<CodeIndexWorkspace | null>(null);
  const [confirmingEmbedding, setConfirmingEmbedding] = useState<CodeIndexWorkspace | null>(null);
  const [pendingDialog, setPendingDialog] = useState<PendingDialog>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const workspaces = useQuery({
    queryKey: ["session-code-index", workspacePath],
    queryFn: () => service.listCodeIndexWorkspaces(),
    refetchInterval: 3_000,
  });
  const retrieval = useQuery({
    queryKey: ["agents", "onepiece-retrieval-configuration"],
    queryFn: () => service.getRetrievalConfiguration(),
  });
  const workspace = workspaces.data?.find((candidate) => pathKey(candidate.canonicalRoot) === pathKey(workspacePath));

  async function run(operation: () => Promise<unknown>) {
    setBusy(true);
    setError(null);
    try {
      await operation();
      await workspaces.refetch();
      return true;
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
      return false;
    } finally {
      setBusy(false);
    }
  }

  async function save(configuration: CodeIndexConfigurationInput) {
    if (workspace && await run(() => service.saveCodeIndexConfiguration(workspace.workspaceId, configuration))) setEditing(null);
  }

  async function confirmEmbedding() {
    const configuration = retrieval.data;
    if (!workspace || !configuration?.sourceProfileId || !configuration.embeddingModel) return;
    if (await run(() => service.confirmCodeIndexEmbedding(workspace.workspaceId, configuration.sourceProfileId ?? "", configuration.embeddingModel ?? "", workspace.generation))) setConfirmingEmbedding(null);
  }

  function confirmDestructive() {
    const dialog = pendingDialog;
    if (!dialog) return;
    if (dialog.action === "delete") {
      setPendingDialog(null);
      void run(() => service.deleteCodeIndexWorkspace(dialog.workspace.workspaceId));
      return;
    }
    void run(() => service.rebuildCodeIndexWorkspace(dialog.workspace.workspaceId)).then((completed) => {
      if (completed) setPendingDialog(null);
    });
  }

  if (workspaces.isLoading) return <div className="flex items-center justify-center gap-2 p-4 text-xs text-muted-foreground"><LoaderCircle className="h-4 w-4 animate-spin" />{t("agents.globalConfig.loading")}</div>;
  if (!workspace) return <div className="ucd-muted-panel rounded-lg p-3 text-center text-xs text-muted-foreground"><p>{t("codeIndex.currentSessionUnavailable")}</p><Button className="mt-3" onClick={() => void workspaces.refetch()} size="sm" variant="outline"><RefreshCw />{t("codeIndex.action.refresh")}</Button></div>;

  const status = workspace.status;
  const embeddingConfigured = Boolean(retrieval.data?.sourceProfileId && retrieval.data.embeddingModel);
  const channels = resolveCodeIndexChannelStatus(workspace, embeddingConfigured);
  return <>
    <section className="ucd-muted-panel rounded-lg p-3">
      <div className="flex flex-wrap items-center gap-2"><Badge tone={status.phase === "ready" ? "success" : status.phase === "degraded" ? "danger" : "warning"}>{t(`codeIndex.phase.${status.phase}`)}</Badge><Badge tone="muted">{t(`codeIndex.mode.${workspace.mode}`)}</Badge></div>
      <p className="mt-2 break-all text-[11px] text-muted-foreground">{workspace.canonicalRoot}</p>
      <div className="mt-3 grid gap-3"><Progress label={t("codeIndex.progress.files")} processed={status.processedFiles} total={status.totalFiles} /><Progress label={t("codeIndex.progress.chunks")} processed={status.processedChunks} total={status.totalChunks} /></div>
      <div className="mt-3 flex flex-wrap gap-1.5"><Badge tone={channels.local === "ready" ? "success" : "muted"}>{t("codeIndex.channel.local")}: {t(`codeIndex.localState.${channels.local}`)}</Badge><Badge tone={channels.semantic === "ready" ? "success" : "muted"}>{t("codeIndex.channel.semantic")}: {t(`codeIndex.semanticState.${channels.semantic}`)}</Badge></div>
      <dl className="mt-3 grid grid-cols-2 gap-2 text-xs">
        <div><dt className="text-muted-foreground">{t("codeIndex.stat.indexed")}</dt><dd className="font-medium tabular-nums">{status.indexedChunks}</dd></div>
        <div><dt className="text-muted-foreground">{t("codeIndex.stat.pending")}</dt><dd className="font-medium tabular-nums">{status.pendingChunks}</dd></div>
        <div><dt className="text-muted-foreground">{t("codeIndex.stat.failed")}</dt><dd className="font-medium tabular-nums">{status.failedFiles + status.failedChunks}</dd></div>
        <div><dt className="text-muted-foreground">{t("codeIndex.stat.redactions")}</dt><dd className="font-medium tabular-nums">{status.redactionCount}</dd></div>
        <div><dt className="text-muted-foreground">{t("codeIndex.stat.requests")}</dt><dd className="font-medium tabular-nums">{status.estimatedEmbeddingRequests}</dd></div>
      </dl>
      <p className="mt-3 text-[11px] text-muted-foreground">{t("codeIndex.stat.updated")}: {formatAppDateTime(status.updatedAt, i18n.language, { dateStyle: "short", timeStyle: "short" })}</p>
      {status.lastFailureCategory ? <p className="mt-2 text-xs ucd-status-warning">{t("codeIndex.failure", { category: status.lastFailureCategory })}</p> : null}
      {workspace.mode === "semantic" && status.phase === "awaiting_embedding_confirmation" ? <Button className="mt-3 w-full" disabled={busy || !embeddingConfigured} onClick={() => setConfirmingEmbedding(workspace)} size="sm">{t("codeIndex.confirmation.review")}</Button> : null}
      <div className="mt-3 grid grid-cols-5 gap-1 border-t border-border pt-3">
        <Button aria-label={t("codeIndex.action.configure")} disabled={busy} onClick={() => setEditing(workspace)} size="icon" title={t("codeIndex.action.configure")} variant="outline"><Settings2 /></Button>
        <Button aria-label={t("codeIndex.action.refresh")} disabled={busy || !workspace.enabled} onClick={() => void run(() => service.refreshCodeIndexWorkspace(workspace.workspaceId))} size="icon" title={t("codeIndex.action.refresh")} variant="outline"><RefreshCw className={busy ? "animate-spin" : ""} /></Button>
        <Button aria-label={t("codeIndex.action.rebuild")} disabled={busy} onClick={() => setPendingDialog({ workspace, action: "rebuild" })} size="icon" title={t("codeIndex.action.rebuild")} variant="outline"><RotateCcw /></Button>
        <Button aria-label={t("codeIndex.action.disable")} disabled={busy || !workspace.enabled} onClick={() => void run(() => service.disableCodeIndexWorkspace(workspace.workspaceId))} size="icon" title={t("codeIndex.action.disable")} variant="ghost"><Ban /></Button>
        <Button aria-label={t("codeIndex.action.delete")} disabled={busy} onClick={() => setPendingDialog({ workspace, action: "delete" })} size="icon" title={t("codeIndex.action.delete")} variant="ghost"><Trash2 /></Button>
      </div>
      {error ?? workspaces.error ? <p className="mt-3 text-xs ucd-status-warning" role="alert">{error ?? String(workspaces.error)}</p> : null}
    </section>
    {editing ? <CodeIndexConfigurationDialog embeddingModel={retrieval.data?.embeddingModel ?? null} embeddingSource={retrieval.data?.sourceProfileId ?? null} onClose={() => setEditing(null)} onSave={save} pending={busy} workspace={editing} /> : null}
    {confirmingEmbedding && retrieval.data?.sourceProfileId && retrieval.data.embeddingModel ? <CodeEmbeddingConfirmationDialog model={retrieval.data.embeddingModel} onClose={() => setConfirmingEmbedding(null)} onConfirm={() => void confirmEmbedding()} pending={busy} profileId={retrieval.data.sourceProfileId} workspace={confirmingEmbedding} /> : null}
    {pendingDialog ? <CodeIndexDestructiveDialog action={pendingDialog.action} onClose={() => setPendingDialog(null)} onConfirm={confirmDestructive} pending={busy} workspace={pendingDialog.workspace} /> : null}
  </>;
}
