import { useMutation } from "@tanstack/react-query";
import { CircleAlert, History, RotateCcw } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { SkillCuratorService } from "../../../services/skill-curator-service";
import type { CuratorCandidateDetail } from "../../../types/skill-curator";
import { SkillCuratorDecisionDialog } from "./skill-curator-decision-dialog";
import { actionKey } from "./skill-curator-draft-editor";

export function SkillCuratorActions({ detail, onChanged, service }: {
  detail: CuratorCandidateDetail;
  onChanged: () => Promise<unknown>;
  service: SkillCuratorService;
}) {
  const { t } = useTranslation();
  const [dialog, setDialog] = useState<{ kind: "reject" | "defer"; trigger: HTMLElement } | null>(null);
  const resume = useMutation({
    mutationFn: () => service.resumeSkillCuratorCandidate({
      candidateId: detail.candidateId,
      expectedCandidateRevision: detail.revision,
      expectedCandidateHash: detail.witnessHash,
      expectedPolicyHash: detail.policyWitnessHash,
      expectedDraftRevision: detail.drafts[0]?.revision,
      expectedAssessmentId: detail.assessmentAttemptId,
      idempotencyKey: actionKey("resume", detail.candidateId),
    }),
    onSuccess: async (result) => { if (result.ok) await onChanged(); },
  });
  const retry = useMutation({
    mutationFn: () => service.retrySkillCuratorApplication({
      candidateId: detail.candidateId,
      expectedCandidateRevision: detail.revision,
      idempotencyKey: actionKey("retry", detail.candidateId),
    }),
    onSuccess: async (result) => { if (result.ok) await onChanged(); },
  });
  const activeError = [resume.data, retry.data].find((result) => result && !result.ok);
  const terminal = ["rejected", "applied", "superseded"].includes(detail.state);
  return <section className="rounded-xl border border-border bg-background p-4"><h4 className="text-sm font-semibold">{t("skills.curator.actions.title")}</h4>
    {detail.staleness.length > 0 || detail.state === "superseded" ? <p className="mt-3 flex gap-2 rounded-md border border-warning/30 bg-warning/5 p-3 text-xs leading-5"><CircleAlert className="mt-0.5 h-4 w-4 shrink-0 text-warning" />{t(detail.state === "superseded" ? "skills.curator.actions.superseded" : "skills.curator.actions.stale", { reasons: detail.staleness.join(", ") })}</p> : null}
    {detail.state === "apply_failed" ? <p className="mt-3 rounded-md border border-destructive/30 bg-destructive/5 p-3 text-xs text-destructive">{t("skills.curator.actions.applyFailed", { code: detail.application?.failureCode ?? "application_failed" })}</p> : null}
    {detail.state === "applied" && detail.application?.overlayHistoryId ? <a className="mt-3 flex min-h-9 items-center gap-2 rounded-md border border-border px-3 text-xs font-medium text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" href={overlayHistoryPath(detail)}><History className="h-4 w-4" />{t("skills.curator.actions.overlayHistory", { id: detail.application.overlayHistoryId })}</a> : null}
    <div className="mt-4 flex flex-wrap gap-2">{!terminal && detail.state !== "deferred" ? <><Button onClick={(event) => setDialog({ kind: "reject", trigger: event.currentTarget })} size="sm" variant="outline">{t("skills.curator.decision.reject")}</Button><Button onClick={(event) => setDialog({ kind: "defer", trigger: event.currentTarget })} size="sm" variant="outline">{t("skills.curator.decision.defer")}</Button></> : null}{detail.state === "deferred" ? <Button disabled={resume.isPending} onClick={() => resume.mutate()} size="sm"><RotateCcw />{t("skills.curator.actions.resume")}</Button> : null}{detail.state === "apply_failed" ? <Button disabled={retry.isPending} onClick={() => retry.mutate()} size="sm" variant="outline"><RotateCcw />{t("skills.curator.actions.retry")}</Button> : null}</div>
    {activeError && !activeError.ok ? <p className="mt-3 text-xs text-destructive" role="alert">{t("skills.curator.actions.error", { code: activeError.error.reasonCode ?? activeError.error.code })}</p> : null}
    {dialog ? <SkillCuratorDecisionDialog detail={detail} kind={dialog.kind} onChanged={onChanged} onClose={() => setDialog(null)} returnFocus={dialog.trigger} service={service} /> : null}
  </section>;
}

function overlayHistoryPath(detail: CuratorCandidateDetail) {
  const params = new URLSearchParams({
    section: "skills",
    skillWorkspace: "inventory",
    skill: detail.targetSkillId,
    workspace: detail.workspaceId,
    overlayHistory: detail.application?.overlayHistoryId ?? "",
  });
  return `/settings?${params.toString()}`;
}
