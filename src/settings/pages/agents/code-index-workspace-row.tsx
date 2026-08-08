import { Ban, RefreshCw, RotateCcw, Settings2, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { formatAppDateTime } from "../../../i18n/format";
import { i18n } from "../../../i18n";
import type { CodeIndexPhase, CodeIndexWorkspace } from "../../../types/code-index";

const activePhases = new Set<CodeIndexPhase>(["scanning", "parsing", "embedding", "cancelling"]);

function phaseTone(phase: CodeIndexPhase): "success" | "warning" | "danger" | "muted" {
  if (phase === "ready") return "success";
  if (phase === "degraded" || phase === "unavailable") return "danger";
  if (activePhases.has(phase) || phase === "awaiting_embedding_confirmation") return "warning";
  return "muted";
}

function Progress({ label, processed, total }: { label: string; processed: number; total: number }) {
  return (
    <div className="min-w-0">
      <div className="mb-1 flex items-center justify-between gap-3 text-xs">
        <span className="text-muted-foreground">{label}</span>
        <span className="tabular-nums">{processed}/{total}</span>
      </div>
      <progress aria-label={label} className="h-1.5 w-full accent-primary" max={Math.max(total, 1)} value={processed} />
    </div>
  );
}

export function CodeIndexWorkspaceRow({
  workspace,
  busy,
  onConfigure,
  onConfirmEmbedding,
  onDelete,
  onDisable,
  onRebuild,
  onRefresh,
}: {
  workspace: CodeIndexWorkspace;
  busy: boolean;
  onConfigure: () => void;
  onConfirmEmbedding: () => void;
  onDelete: () => void;
  onDisable: () => void;
  onRebuild: () => void;
  onRefresh: () => void;
}) {
  const { t } = useTranslation();
  const status = workspace.status;
  return (
    <article className="border-t border-border px-4 py-4 first:border-t-0 sm:px-5">
      <div className="flex flex-col gap-3 xl:flex-row xl:items-start">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <h4 className="truncate text-sm font-semibold">{workspace.displayName}</h4>
            <Badge tone={phaseTone(status.phase)}>{t(`codeIndex.phase.${status.phase}`)}</Badge>
          </div>
          <p className="mt-1 truncate text-xs text-muted-foreground" title={workspace.canonicalRoot}>{workspace.canonicalRoot}</p>
          <p className="mt-1 truncate font-mono text-[11px] text-muted-foreground" title={workspace.workspaceId}>{workspace.workspaceId}</p>
        </div>
        <div className="flex flex-wrap gap-1.5">
          <Button aria-label={t("codeIndex.action.configure")} disabled={busy} onClick={onConfigure} size="icon" title={t("codeIndex.action.configure")} variant="outline"><Settings2 /></Button>
          <Button aria-label={t("codeIndex.action.refresh")} disabled={busy || !workspace.enabled} onClick={onRefresh} size="icon" title={t("codeIndex.action.refresh")} variant="outline"><RefreshCw className={busy ? "animate-spin" : ""} /></Button>
          <Button aria-label={t("codeIndex.action.rebuild")} disabled={busy} onClick={onRebuild} size="icon" title={t("codeIndex.action.rebuild")} variant="outline"><RotateCcw /></Button>
          {workspace.enabled ? <Button aria-label={t("codeIndex.action.disable")} disabled={busy} onClick={onDisable} size="icon" title={t("codeIndex.action.disable")} variant="ghost"><Ban /></Button> : null}
          <Button aria-label={t("codeIndex.action.delete")} disabled={busy} onClick={onDelete} size="icon" title={t("codeIndex.action.delete")} variant="ghost"><Trash2 /></Button>
        </div>
      </div>

      <div className="mt-4 grid gap-4 md:grid-cols-2">
        <Progress label={t("codeIndex.progress.files")} processed={status.processedFiles} total={status.totalFiles} />
        <Progress label={t("codeIndex.progress.chunks")} processed={status.processedChunks} total={status.totalChunks} />
      </div>
      <dl className="mt-4 grid grid-cols-2 gap-x-4 gap-y-2 text-xs sm:grid-cols-4 xl:grid-cols-6">
        <div><dt className="text-muted-foreground">{t("codeIndex.stat.pending")}</dt><dd className="mt-0.5 font-medium tabular-nums">{status.pendingChunks}</dd></div>
        <div><dt className="text-muted-foreground">{t("codeIndex.stat.indexed")}</dt><dd className="mt-0.5 font-medium tabular-nums">{status.indexedChunks}</dd></div>
        <div><dt className="text-muted-foreground">{t("codeIndex.stat.failed")}</dt><dd className="mt-0.5 font-medium tabular-nums">{status.failedFiles + status.failedChunks}</dd></div>
        <div><dt className="text-muted-foreground">{t("codeIndex.stat.redactions")}</dt><dd className="mt-0.5 font-medium tabular-nums">{status.redactionCount}</dd></div>
        <div><dt className="text-muted-foreground">{t("codeIndex.stat.requests")}</dt><dd className="mt-0.5 font-medium tabular-nums">{status.estimatedEmbeddingRequests}</dd></div>
        <div><dt className="text-muted-foreground">{t("codeIndex.stat.updated")}</dt><dd className="mt-0.5 font-medium">{formatAppDateTime(status.updatedAt, i18n.language, { dateStyle: "short", timeStyle: "short" })}</dd></div>
      </dl>
      {status.phase === "awaiting_embedding_confirmation" ? (
        <div className="mt-4 flex flex-wrap items-center justify-between gap-2 border-t border-border pt-3">
          <p className="text-xs text-muted-foreground">{t("codeIndex.confirmation.required")}</p>
          <Button disabled={busy} onClick={onConfirmEmbedding} size="sm">{t("codeIndex.confirmation.review")}</Button>
        </div>
      ) : null}
      {status.lastFailureCategory ? <p className="mt-3 text-xs ucd-status-warning">{t("codeIndex.failure", { category: status.lastFailureCategory })}</p> : null}
    </article>
  );
}
