import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import { agentService } from "../../../services/runtime-agent-client";
import { normalizeSkillOverlayError } from "../../../services/skill-overlay-error";
import type { SkillOverlayDetail, SkillOverlayResourceSummary, SkillOverlayTargetInput } from "../../../types/skill-overlay";
import { witnessesFor } from "./skill-overlay-mutation-dialog";

export type ResourceStateAction = "disable" | "revert";

export function SkillOverlayResourceStateDialog({
  action,
  detail,
  resource,
  target,
  returnFocus,
  onClose,
  onCommitted,
  onRefresh,
}: {
  action: ResourceStateAction;
  detail: SkillOverlayDetail;
  resource: SkillOverlayResourceSummary;
  target: SkillOverlayTargetInput;
  returnFocus: HTMLElement | null;
  onClose: () => void;
  onCommitted: () => void;
  onRefresh: () => Promise<unknown> | void;
}) {
  const { t } = useTranslation();
  const [pending, setPending] = useState<"commit" | "reload" | null>(null);
  const [error, setError] = useState<{ message: string; stale: boolean } | null>(null);

  async function commit() {
    if (detail.summary.pinned) return;
    setPending("commit");
    setError(null);
    const input = { target, witnesses: witnessesFor(detail, target), mutationId: resource.mutationId, mutationKind: "supportingFile" as const };
    try {
      if (action === "disable") await agentService.disableSkillOverlayMutation(input);
      else await agentService.revertSkillOverlayMutation(input);
      onCommitted();
      onClose();
    } catch (caught) {
      const normalized = normalizeSkillOverlayError(caught);
      setError({ message: normalized.message, stale: normalized.kind === "stale" });
    } finally {
      setPending(null);
    }
  }

  async function reload() {
    setPending("reload");
    try {
      await onRefresh();
      setError(null);
    } catch (caught) {
      setError({ message: caught instanceof Error ? caught.message : String(caught), stale: true });
    } finally {
      setPending(null);
    }
  }

  const busy = pending !== null;
  return <ApplicationDialog closeDisabled={busy} description={t(`skills.overlay.resource.${action}Description`)} onClose={onClose} returnFocus={returnFocus} title={t(`skills.overlay.resource.${action}Title`)}>
    <dl className="grid gap-2 rounded-md border border-border bg-muted/20 p-3 text-xs sm:grid-cols-2">
      <Item label={t("skills.overlay.resource.path")} value={resource.logicalPath} />
      <Item label={t("skills.overlay.resource.scopeRevision")} value={`${t(`skills.overlay.scope.${resource.effectiveScope}`)} · r${witnessesFor(detail, target).expectedOverlayRevision ?? 0}`} />
      <Item label={t("skills.overlay.resource.mediaType")} value={resource.mediaType} />
      <Item label={t("skills.overlay.resource.hash")} value={resource.contentHash} />
    </dl>
    <p className="mt-3 text-xs leading-5 text-muted-foreground">{t(`skills.overlay.resource.${action}AuditHint`)}</p>
    {error ? <div className="mt-3 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive" role="alert"><p>{error.message}</p>{error.stale ? <Button className="mt-2" disabled={busy} onClick={() => void reload()} size="sm" variant="outline">{t(pending === "reload" ? "skills.overlay.mutation.reloading" : "skills.overlay.mutation.reload")}</Button> : null}</div> : null}
    <div className="mt-5 flex flex-col-reverse gap-2 border-t border-border pt-4 sm:flex-row sm:justify-end">
      {busy ? <span className="mr-auto text-sm text-muted-foreground" role="status">{t("skills.dialog.pending")}</span> : null}
      <Button disabled={busy} onClick={onClose} variant="outline">{t("skills.dialog.cancel")}</Button>
      <Button disabled={detail.summary.pinned || busy} onClick={() => void commit()}>{t(`skills.overlay.resource.${action}`)}</Button>
    </div>
  </ApplicationDialog>;
}

function Item({ label, value }: { label: string; value: string }) {
  return <div className="min-w-0"><dt className="text-muted-foreground">{label}</dt><dd className="mt-1 break-all font-mono">{value}</dd></div>;
}
