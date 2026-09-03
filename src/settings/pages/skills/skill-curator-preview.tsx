import { useMutation } from "@tanstack/react-query";
import { CheckCircle2, Diff, TimerReset, TriangleAlert } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { SkillCuratorService } from "../../../services/skill-curator-service";
import type { CuratorCandidateDetail, CuratorDiffProjection } from "../../../types/skill-curator";
import { actionKey } from "./skill-curator-draft-editor";

type DiffKind = "baseToCurrent" | "currentToProposed" | "baseToProposed";

export function SkillCuratorPreview({ detail, onChanged, service }: {
  detail: CuratorCandidateDetail;
  onChanged: () => Promise<unknown>;
  service: SkillCuratorService;
}) {
  const { i18n, t } = useTranslation();
  const [selection, setSelection] = useState<DiffKind>("baseToProposed");
  const [confirmed, setConfirmed] = useState(false);
  const previewMutation = useMutation({
    mutationFn: () => service.previewSkillCuratorCandidate({
      candidateId: detail.candidateId,
      expectedCandidateRevision: detail.revision,
      expectedDraftRevision: detail.drafts[0]?.revision ?? 0,
      expectedAssessmentId: detail.assessmentAttemptId,
      idempotencyKey: actionKey("preview", detail.candidateId),
    }),
    onSuccess: async () => { await onChanged(); },
  });
  const approve = useMutation({
    mutationFn: () => service.approveSkillCuratorCandidate({
      candidateId: detail.candidateId,
      expectedCandidateRevision: detail.revision,
      confirmedPreviewHash: detail.currentPreview?.witnessHash ?? "",
      confirmedEffectiveDiffHash: detail.currentPreview?.effectiveDiffHash ?? "",
      idempotencyKey: actionKey("approve", detail.candidateId),
    }),
    onSuccess: async () => { await onChanged(); },
  });
  const preview = detail.currentPreview;
  const expired = Boolean(preview && (preview.invalidatedAtMs || Date.now() >= preview.expiresAtMs));
  useEffect(() => setConfirmed(false), [preview?.witnessHash]);
  const resultError = [previewMutation.data, approve.data].find((result) => result && !result.ok);
  return <section className="rounded-xl border border-border bg-background p-4" aria-labelledby="curator-preview-title">
    <div className="flex flex-wrap items-start justify-between gap-3"><div><h4 className="flex items-center gap-2 text-sm font-semibold" id="curator-preview-title"><Diff className="h-4 w-4 text-primary" />{t("skills.curator.preview.title")}</h4><p className="mt-1 text-xs leading-5 text-muted-foreground">{t("skills.curator.preview.description")}</p></div><Button disabled={!detail.draftReady || previewMutation.isPending || detail.state !== "ready_for_review"} onClick={() => previewMutation.mutate()} size="sm" variant="outline">{previewMutation.isPending ? t("skills.curator.preview.loading") : t("skills.curator.preview.create")}</Button></div>
    {preview ? <><div className="mt-3 flex flex-wrap gap-2">{(["baseToCurrent", "currentToProposed", "baseToProposed"] as DiffKind[]).map((kind) => <button aria-pressed={selection === kind} className={`rounded-md border px-2.5 py-2 text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${selection === kind ? "border-primary bg-primary text-primary-foreground" : "border-border"}`} key={kind} onClick={() => setSelection(kind)} type="button">{t(`skills.curator.preview.${kind}`)}</button>)}</div><DiffProjection projection={preview.diffs[selection]} />
      <div className="mt-3 grid gap-2 text-xs sm:grid-cols-2 xl:grid-cols-3"><Validation label={t("skills.curator.preview.scan")} passed={preview.validation.scanPassed} /><Validation label={t("skills.curator.preview.trust")} passed={preview.validation.trusted} /><Validation label={t("skills.curator.preview.notPinned")} passed={!preview.validation.pinned} /><Validation label={t("skills.curator.preview.noConflicts")} passed={preview.validation.conflictCount === 0 && preview.validation.conflictsComplete} /><Validation label={t("skills.curator.preview.rulesComplete")} passed={preview.validation.rulesComplete} /><Validation label={t("skills.curator.preview.canCommit")} passed={preview.validation.canCommit} /></div>
      <p className={`mt-3 flex items-center gap-2 text-xs ${expired ? "text-destructive" : "text-muted-foreground"}`}><TimerReset className="h-4 w-4" />{t(expired ? "skills.curator.preview.expired" : "skills.curator.preview.expires", { time: new Intl.DateTimeFormat(i18n.language, { dateStyle: "medium", timeStyle: "short" }).format(preview.expiresAtMs) })}</p>
      <label className="mt-3 flex items-start gap-2 rounded-md border border-primary/30 bg-primary/5 p-3 text-xs leading-5"><input checked={confirmed} className="mt-1 h-4 w-4 accent-primary" disabled={expired || !preview.validation.canCommit} onChange={(event) => setConfirmed(event.target.checked)} type="checkbox" /><span>{t("skills.curator.preview.confirm")}</span></label>
      <div className="mt-3 flex justify-end"><Button disabled={!confirmed || expired || !preview.validation.canCommit || approve.isPending} onClick={() => approve.mutate()}>{approve.isPending ? t("skills.curator.preview.applying") : t("skills.curator.preview.approve")}</Button></div>
    </> : <p className="mt-3 rounded-md border border-dashed border-border p-3 text-xs text-muted-foreground">{t("skills.curator.preview.none")}</p>}
    {resultError && !resultError.ok ? <p className="mt-3 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-xs text-destructive" role="alert">{t("skills.curator.preview.error", { code: resultError.error.reasonCode ?? resultError.error.code })}</p> : null}
  </section>;
}

function DiffProjection({ projection }: { projection: CuratorDiffProjection }) {
  const { t } = useTranslation();
  return <div className="mt-3 space-y-2"><div className="flex gap-2"><Badge tone="success">+{projection.addedCharacters}</Badge><Badge tone="danger">−{projection.removedCharacters}</Badge>{!projection.complete ? <Badge tone="warning">{t("skills.curator.preview.truncated")}</Badge> : null}</div>{projection.hunks.map((hunk, index) => <article className="overflow-hidden rounded-md border border-border" key={`${hunk.label}-${index}`}><p className="bg-muted/40 px-3 py-2 text-xs font-medium">{hunk.label}</p><div className="grid md:grid-cols-2"><DiffText content={hunk.before.content} kind="before" /><DiffText content={hunk.after.content} kind="after" /></div></article>)}</div>;
}

function DiffText({ content, kind }: { content: string; kind: "before" | "after" }) { const { t } = useTranslation(); return <div className={`min-w-0 p-3 ${kind === "before" ? "bg-destructive/5" : "bg-primary/5"}`}><p className="mb-2 text-[10px] font-semibold uppercase text-muted-foreground">{t(`skills.curator.preview.${kind}`)}</p><pre className="max-h-52 overflow-auto whitespace-pre-wrap break-words font-mono text-xs">{content || "∅"}</pre></div>; }
function Validation({ label, passed }: { label: string; passed: boolean }) { return <p className={`flex items-center gap-2 rounded-md border p-2 ${passed ? "border-success/30 bg-success/5" : "border-warning/30 bg-warning/5"}`}>{passed ? <CheckCircle2 className="h-4 w-4 text-success" /> : <TriangleAlert className="h-4 w-4 text-warning" />}{label}</p>; }
