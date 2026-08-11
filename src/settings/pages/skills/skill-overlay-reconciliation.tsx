import { GitMerge } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import { agentService } from "../../../services/runtime-agent-client";
import { normalizeSkillOverlayError } from "../../../services/skill-overlay-error";
import type { SkillOverlayDetail, SkillOverlayTargetInput } from "../../../types/skill-overlay";
import type { SkillOverlayConflictResolutionInput, SkillOverlayReconciliationPreview } from "../../../types/skill-overlay-reconciliation";
import { SkillOverlayConflictEditor, type ConflictDrafts } from "./skill-overlay-conflict-editor";
import { SKILL_OVERLAY_PINNED_DESCRIPTION_ID } from "./skill-overlay-pinned-notice";
import { SkillOverlayReconciliationComparison } from "./skill-overlay-reconciliation-comparison";

interface PreviewRecord {
  fingerprint: string;
  result: SkillOverlayReconciliationPreview;
}

export function SkillOverlayReconciliationAction({ detail, target, onCommitted, onRefresh }: {
  detail: SkillOverlayDetail;
  target: SkillOverlayTargetInput;
  onCommitted: () => void;
  onRefresh: () => Promise<unknown> | void;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null);
  if (!detail.summary.needsReconcile && !detail.conflicts.some((conflict) => conflict.state === "active")) return null;
  return <>
    <div className="rounded-md border border-warning/40 bg-warning/10 p-3 sm:flex sm:items-center sm:justify-between sm:gap-3">
      <div><p className="flex items-center gap-2 text-sm font-semibold"><GitMerge className="h-4 w-4" />{t("skills.overlay.reconcile.bannerTitle")}</p><p className="mt-1 text-xs leading-5 text-muted-foreground">{t("skills.overlay.reconcile.bannerDescription")}</p></div>
      <Button aria-describedby={detail.summary.pinned ? SKILL_OVERLAY_PINNED_DESCRIPTION_ID : undefined} className="mt-3 min-h-11 shrink-0 sm:mt-0 sm:min-h-9" disabled={detail.summary.pinned} onClick={() => setOpen(true)} ref={trigger}>{t("skills.overlay.reconcile.open")}</Button>
    </div>
    {open ? <SkillOverlayReconciliationDialog detail={detail} onClose={() => setOpen(false)} onCommitted={onCommitted} onRefresh={onRefresh} returnFocus={trigger.current} target={target} /> : null}
  </>;
}

export function SkillOverlayReconciliationDialog({ detail, target, returnFocus, onClose, onCommitted, onRefresh }: {
  detail: SkillOverlayDetail;
  target: SkillOverlayTargetInput;
  returnFocus: HTMLElement | null;
  onClose: () => void;
  onCommitted: () => void;
  onRefresh: () => Promise<unknown> | void;
}) {
  const { t } = useTranslation();
  const [drafts, setDrafts] = useState<ConflictDrafts>({});
  const [comparison, setComparison] = useState<SkillOverlayReconciliationPreview | null>(null);
  const [preview, setPreview] = useState<PreviewRecord | null>(null);
  const [acknowledged, setAcknowledged] = useState(false);
  const [error, setError] = useState<{ message: string; stale: boolean } | null>(null);
  const [pending, setPending] = useState<"load" | "preview" | "commit" | "reload" | null>("load");
  const { skillId, scope, workspacePath } = target;
  const requestTarget = useMemo(
    () => workspacePath === undefined ? { skillId, scope } : { skillId, scope, workspacePath },
    [skillId, scope, workspacePath],
  );
  const scopeRevision = detail.summary.scopes.find((summary) => summary.scope === scope)?.revision ?? null;
  const witnesses = useMemo(() => ({
    expectedOverlayRevision: scopeRevision,
    expectedBaseInstructionHash: detail.summary.baseInstructionHash,
    expectedBasePackageHash: detail.summary.basePackageHash,
    expectedPayloadHash: null,
    expectedPinned: detail.summary.pinned,
  }), [scopeRevision, detail.summary.baseInstructionHash, detail.summary.basePackageHash, detail.summary.pinned]);
  const choices = useMemo(() => choicesFor(drafts), [drafts]);
  const fingerprint = JSON.stringify({ target: requestTarget, witnesses, choices });
  const latestFingerprint = useRef(fingerprint);
  latestFingerprint.current = fingerprint;
  const currentPreview = preview?.fingerprint === fingerprint ? preview.result : null;

  useEffect(() => {
    if (detail.summary.pinned) {
      setPending(null);
      return;
    }
    let active = true;
    setPending("load");
    setError(null);
    agentService.previewSkillOverlayReconciliation({ target: requestTarget, witnesses, choices: [] })
      .then((result) => { if (active) setComparison(result); })
      .catch((caught: unknown) => { if (active) setError(normalizedError(caught)); })
      .finally(() => { if (active) setPending(null); });
    return () => { active = false; };
  }, [detail.summary.pinned, requestTarget, witnesses]);

  function updateDrafts(value: ConflictDrafts) {
    setDrafts(value);
    setAcknowledged(false);
    setError(null);
  }

  async function requestPreview() {
    if (detail.summary.pinned) return;
    const requestedFingerprint = fingerprint;
    setPending("preview");
    setError(null);
    try {
      const result = await agentService.previewSkillOverlayReconciliation({ target: requestTarget, witnesses, choices });
      setComparison(result);
      if (latestFingerprint.current === requestedFingerprint) setPreview({ fingerprint: requestedFingerprint, result });
    } catch (caught) {
      setError(normalizedError(caught));
    } finally {
      setPending(null);
    }
  }

  async function commit() {
    if (detail.summary.pinned || !currentPreview?.canCommit || !currentPreview.finalDiffComplete || !acknowledged) return;
    setPending("commit");
    setError(null);
    try {
      await agentService.reconcileSkillOverlay({ target: requestTarget, witnesses: currentPreview.witnesses, choices });
      onCommitted();
      onClose();
    } catch (caught) {
      setPreview(null);
      setAcknowledged(false);
      setError(normalizedError(caught));
    } finally {
      setPending(null);
    }
  }

  async function reload() {
    setPending("reload");
    try {
      await onRefresh();
      setPreview(null);
      setAcknowledged(false);
      setError(null);
    } catch (caught) {
      setError({ message: caught instanceof Error ? caught.message : String(caught), stale: true });
    } finally {
      setPending(null);
    }
  }

  const conflicts = comparison?.conflictChoices ?? detail.conflicts.map((conflict) => ({ conflict, selectedResolution: null }));
  const activeConflicts = conflicts.filter(({ conflict }) => conflict.state === "active");
  const choicesComplete = activeConflicts.every(({ conflict }) => {
    const draft = drafts[conflict.id];
    return draft?.resolution === "ignore" || (draft?.resolution === "editPatch" && draft.oldString.length > 0);
  });
  const busy = pending !== null;
  return <ApplicationDialog closeDisabled={busy} description={t("skills.overlay.reconcile.description")} maxWidth="max-w-6xl" onClose={onClose} returnFocus={returnFocus} title={t("skills.overlay.reconcile.title")}>
    <div className="space-y-4">
      {comparison ? <SkillOverlayReconciliationComparison preview={currentPreview ?? comparison} previewCurrent={Boolean(currentPreview)} /> : <p className="rounded-md border border-dashed border-border p-3 text-sm text-muted-foreground" role="status">{t("skills.overlay.reconcile.loadingComparison")}</p>}
      {activeConflicts.length > 0 ? <SkillOverlayConflictEditor conflicts={activeConflicts} detail={detail} disabled={detail.summary.pinned} drafts={drafts} onChange={updateDrafts} /> : <p className="rounded-md border border-border bg-muted/10 p-3 text-sm">{t("skills.overlay.reconcile.cleanReplay")}</p>}
      {error ? <div className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive" role="alert"><p>{error.message}</p>{error.stale ? <Button className="mt-2" disabled={busy} onClick={() => void reload()} size="sm" variant="outline">{t("skills.overlay.reconcile.reload")}</Button> : null}</div> : null}
      {currentPreview?.canCommit && currentPreview.finalDiffComplete ? <label className="flex min-h-11 items-start gap-3 rounded-md border border-border px-3 py-2 text-sm"><input checked={acknowledged} className="mt-0.5 h-4 w-4" disabled={detail.summary.pinned} onChange={(event) => setAcknowledged(event.target.checked)} type="checkbox" /><span>{t("skills.overlay.reconcile.acknowledge")}</span></label> : null}
    </div>
    <div className="mt-5 flex flex-col-reverse gap-2 border-t border-border pt-4 sm:flex-row sm:items-center sm:justify-end">
      {busy ? <span className="mr-auto text-sm text-muted-foreground" role="status">{t("skills.dialog.pending")}</span> : null}
      <Button disabled={busy} onClick={onClose} variant="outline">{t("skills.dialog.cancel")}</Button>
      <Button disabled={detail.summary.pinned || busy || !comparison || !choicesComplete} onClick={() => void requestPreview()} variant="outline">{t("skills.overlay.reconcile.preview")}</Button>
      <Button disabled={detail.summary.pinned || busy || !acknowledged || !currentPreview?.canCommit || !currentPreview.finalDiffComplete} onClick={() => void commit()}>{t("skills.overlay.reconcile.commit")}</Button>
    </div>
  </ApplicationDialog>;
}

function choicesFor(drafts: ConflictDrafts): SkillOverlayConflictResolutionInput[] {
  const choices: SkillOverlayConflictResolutionInput[] = [];
  for (const [conflictId, draft] of Object.entries(drafts)) {
    if (draft.resolution === "ignore") choices.push({ conflictId, resolution: "ignore" });
    if (draft.resolution === "editPatch" && draft.oldString.length > 0) {
      choices.push({ conflictId, resolution: "editPatch", oldString: draft.oldString, newString: draft.newString, replaceAll: draft.replaceAll });
    }
  }
  return choices;
}

function normalizedError(caught: unknown) {
  const normalized = normalizeSkillOverlayError(caught);
  return { message: normalized.message, stale: normalized.kind === "stale" };
}
