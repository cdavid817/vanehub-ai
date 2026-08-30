import { useMutation } from "@tanstack/react-query";
import { useState, type RefObject } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import type { SkillCuratorService } from "../../../services/skill-curator-service";
import type { CuratorCandidateDetail } from "../../../types/skill-curator";
import { actionKey } from "./skill-curator-draft-editor";

type DecisionKind = "reject" | "defer";

export function SkillCuratorDecisionDialog({ detail, kind, onChanged, onClose, returnFocus, service }: {
  detail: CuratorCandidateDetail;
  kind: DecisionKind;
  onChanged: () => Promise<unknown>;
  onClose: () => void;
  returnFocus: HTMLElement | null;
  service: SkillCuratorService;
}) {
  const { t } = useTranslation();
  const [reason, setReason] = useState("");
  const [note, setNote] = useState("");
  const [reviewAfter, setReviewAfter] = useState("");
  const mutation = useMutation({
    mutationFn: () => kind === "reject"
      ? service.rejectSkillCuratorCandidate({
        candidateId: detail.candidateId,
        expectedCandidateRevision: detail.revision,
        idempotencyKey: actionKey("reject", detail.candidateId),
        reason: reason as Parameters<SkillCuratorService["rejectSkillCuratorCandidate"]>[0]["reason"],
        ...(note.trim() ? { note } : {}),
      })
      : service.deferSkillCuratorCandidate({
        candidateId: detail.candidateId,
        expectedCandidateRevision: detail.revision,
        idempotencyKey: actionKey("defer", detail.candidateId),
        reason: reason as Parameters<SkillCuratorService["deferSkillCuratorCandidate"]>[0]["reason"],
        ...(note.trim() ? { note } : {}),
        ...(reviewAfter ? { reviewAfterMs: new Date(reviewAfter).getTime() } : {}),
      }),
    onSuccess: async (result) => {
      if (!result.ok) return;
      await onChanged();
      onClose();
    },
  });
  const options = kind === "reject"
    ? ["incorrect_target", "unsupported_lesson", "duplicate", "too_risky", "not_useful", "other"]
    : ["need_more_evidence", "need_expert_review", "waiting_for_change", "lower_priority", "other"];
  const reviewTime = reviewAfter ? new Date(reviewAfter).getTime() : undefined;
  const reviewValid = kind === "reject" || reviewTime === undefined || (Number.isFinite(reviewTime) && reviewTime >= Date.now() + 86_400_000);
  const error = mutation.data && !mutation.data.ok ? mutation.data.error : undefined;
  return <ApplicationDialog closeDisabled={mutation.isPending} description={t(`skills.curator.decision.${kind}Description`)} maxWidth="max-w-lg" onClose={onClose} returnFocus={returnFocus} title={t(`skills.curator.decision.${kind}Title`)}>
    <div className="space-y-4"><label className="block text-xs text-muted-foreground"><span>{t("skills.curator.decision.reason")}</span><select className={controlClass} data-dialog-autofocus onChange={(event) => setReason(event.target.value)} required value={reason}><option value="">{t("skills.curator.decision.chooseReason")}</option>{options.map((option) => <option key={option} value={option}>{t(`skills.curator.decision.reason.${option}`)}</option>)}</select></label>
      <label className="block text-xs text-muted-foreground"><span>{t("skills.curator.decision.note")}</span><textarea className={`${controlClass} min-h-24 py-2`} maxLength={1000} onChange={(event) => setNote(event.target.value)} value={note} /><span className="mt-1 block text-right tabular-nums">{note.length} / 1000</span></label>
      {kind === "defer" ? <label className="block text-xs text-muted-foreground"><span>{t("skills.curator.decision.reviewAfter")}</span><input className={controlClass} min={new Date(Date.now() + 86_400_000).toISOString().slice(0, 16)} onChange={(event) => setReviewAfter(event.target.value)} type="datetime-local" value={reviewAfter} />{!reviewValid ? <span className="mt-1 block text-destructive">{t("skills.curator.decision.reviewAfterError")}</span> : <span className="mt-1 block">{t("skills.curator.decision.manualResume")}</span>}</label> : null}
      {error ? <p className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-xs text-destructive" role="alert">{t("skills.curator.decision.error", { code: error.reasonCode ?? error.code })}</p> : null}
      <div className="flex justify-end gap-2 border-t border-border pt-4"><Button disabled={mutation.isPending} onClick={onClose} variant="outline">{t("skills.curator.decision.cancel")}</Button><Button disabled={!reason || !reviewValid || mutation.isPending} onClick={() => mutation.mutate()}>{mutation.isPending ? t("skills.curator.decision.saving") : t(`skills.curator.decision.${kind}`)}</Button></div>
    </div>
  </ApplicationDialog>;
}

const controlClass = "mt-1 h-9 w-full rounded-md border border-border bg-background px-2 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring";

// Keeps the dialog return-focus contract visible at call sites without exposing mutations on notifications.
export type CuratorDecisionReturnFocus = RefObject<HTMLElement | null>;
