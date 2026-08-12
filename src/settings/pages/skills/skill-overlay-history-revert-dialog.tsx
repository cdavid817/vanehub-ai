import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import { agentService } from "../../../services/runtime-agent-client";
import { normalizeSkillOverlayError } from "../../../services/skill-overlay-error";
import type { SkillOverlayDetail, SkillOverlayMutationSummary, SkillOverlayTargetInput } from "../../../types/skill-overlay";
import { witnessesFor } from "./skill-overlay-mutation-dialog";

export function SkillOverlayHistoryRevertDialog({ detail, mutation, target, returnFocus, onClose, onCommitted, onRefresh, onReverted }: {
  detail: SkillOverlayDetail;
  mutation: SkillOverlayMutationSummary;
  target: SkillOverlayTargetInput;
  returnFocus: HTMLElement | null;
  onClose: () => void;
  onCommitted: () => void;
  onRefresh: () => Promise<unknown> | void;
  onReverted: (revision: number) => void;
}) {
  const { t } = useTranslation();
  const [pending, setPending] = useState<"commit" | "reload" | null>(null);
  const [error, setError] = useState<{ message: string; stale: boolean } | null>(null);
  const scopedTarget = targetForMutation(target, mutation);

  async function commit() {
    if (detail.summary.pinned) return;
    setPending("commit");
    setError(null);
    try {
      const outcome = await agentService.revertSkillOverlayMutation({
        target: scopedTarget,
        witnesses: witnessesFor(detail, scopedTarget),
        mutationId: mutation.id,
        mutationKind: mutation.kind,
      });
      onReverted(outcome.committedRevision);
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
  return <ApplicationDialog closeDisabled={busy} description={t("skills.overlay.history.revertDescription")} onClose={onClose} returnFocus={returnFocus} title={t("skills.overlay.history.revertTitle")}>
    <dl className="grid gap-2 rounded-md border border-border bg-muted/20 p-3 text-xs sm:grid-cols-2">
      <Item label={t("skills.overlay.history.mutationId")} value={mutation.id} />
      <Item label={t("skills.overlay.history.mutationKind")} value={t(`skills.overlay.mutations.${mutation.kind}`)} />
      <Item label={t("skills.overlay.history.scopeLabel")} value={t(`skills.overlay.scope.${mutation.scope}`)} />
      <Item label={t("skills.overlay.history.currentRevision")} value={`r${witnessesFor(detail, scopedTarget).expectedOverlayRevision ?? 0}`} />
    </dl>
    <p className="mt-3 rounded-md border border-warning/40 bg-warning/10 p-3 text-xs leading-5">{t("skills.overlay.history.revertAuditHint")}</p>
    {error ? <div className="mt-3 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive" role="alert">
      <p>{error.message}</p>
      {error.stale ? <Button className="mt-2" disabled={busy} onClick={() => void reload()} size="sm" variant="outline">{t(pending === "reload" ? "skills.overlay.mutation.reloading" : "skills.overlay.mutation.reload")}</Button> : null}
    </div> : null}
    <div className="mt-5 flex flex-col-reverse gap-2 border-t border-border pt-4 sm:flex-row sm:justify-end">
      {busy ? <span className="mr-auto text-sm text-muted-foreground" role="status">{t("skills.dialog.pending")}</span> : null}
      <Button disabled={busy} onClick={onClose} variant="outline">{t("skills.dialog.cancel")}</Button>
      <Button disabled={detail.summary.pinned || busy} onClick={() => void commit()}>{t("skills.overlay.history.confirmRevert")}</Button>
    </div>
  </ApplicationDialog>;
}

function Item({ label, value }: { label: string; value: string }) {
  return <div className="min-w-0"><dt className="text-muted-foreground">{label}</dt><dd className="mt-1 break-all font-mono">{value}</dd></div>;
}

function targetForMutation(target: SkillOverlayTargetInput, mutation: SkillOverlayMutationSummary): SkillOverlayTargetInput {
  return { skillId: target.skillId, scope: mutation.scope, workspacePath: mutation.scope === "project" ? target.workspacePath ?? null : null };
}
