import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { BrainCircuit, RefreshCw, ShieldCheck } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { agentService } from "../../../services/runtime-agent-client";
import type { Skill } from "../../../types/skill";
import { AssessmentExplanation, AssessmentHistory } from "./skill-assessment-details";

export function SkillEvolutionAssessment({ skill }: { skill: Skill }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [selectedAttemptId, setSelectedAttemptId] = useState<string | null>(null);
  const [disclosureConfirmed, setDisclosureConfirmed] = useState(false);
  const scope = useMemo(() => ({
    workspace: skill.workspacePath ?? undefined,
    skillId: skill.id,
    includeHistory: true,
    limit: 50,
  }), [skill.id, skill.workspacePath]);
  const pageQuery = useQuery({
    queryKey: ["skill-evolution-assessments", scope.workspace ?? "", scope.skillId],
    queryFn: () => agentService.querySkillEvolutionAssessments(scope),
  });
  const current = pageQuery.data?.items.find((item) => item.isCurrent) ?? pageQuery.data?.items[0];
  const activeAttemptId = selectedAttemptId ?? current?.attemptId;
  const detailQuery = useQuery({
    queryKey: ["skill-evolution-assessment", activeAttemptId],
    queryFn: () => agentService.getSkillEvolutionAssessment(activeAttemptId ?? ""),
    enabled: Boolean(activeAttemptId),
  });
  const policyQuery = useQuery({
    queryKey: ["skill-evolution-assessment-policy"],
    queryFn: () => agentService.getSkillEvolutionAssessmentPolicy(),
  });
  const consent = useMutation({
    mutationFn: (enabled: boolean) => {
      const policy = policyQuery.data;
      if (!policy) throw new Error("assessment-policy-unavailable");
      return agentService.updateSkillEvolutionAssessmentConsent({
        enabled,
        evaluatorPolicyVersion: policy.evaluatorPolicyVersion,
        disclosureVersion: policy.disclosureVersion,
      });
    },
    onSuccess: (policy) => {
      queryClient.setQueryData(["skill-evolution-assessment-policy"], policy);
      setDisclosureConfirmed(false);
    },
  });
  const reassess = useMutation({
    mutationFn: () => {
      if (!current) throw new Error("assessment-missing-seed");
      return agentService.scheduleSkillEvolutionReassessment({ seedId: current.seedId });
    },
    onSuccess: () => void pageQuery.refetch(),
  });

  return <section aria-labelledby="skill-assessment-heading" className="rounded-xl border border-border bg-gradient-to-br from-primary/5 via-background to-background p-3 shadow-sm sm:p-4">
    <header className="flex flex-wrap items-start justify-between gap-3">
      <div className="min-w-0">
        <div className="flex items-center gap-2"><BrainCircuit className="h-4 w-4 text-primary" /><h4 className="text-sm font-semibold" id="skill-assessment-heading">{t("skills.assessment.title")}</h4></div>
        <p className="mt-1 max-w-3xl text-xs leading-5 text-muted-foreground">{t("skills.assessment.description")}</p>
      </div>
      {current ? <Badge tone={statusTone(current.status)}>{t(`skills.assessment.status.${current.status}`)}</Badge> : null}
    </header>

    {pageQuery.isLoading ? <Status text={t("skills.assessment.loading")} /> : null}
    {pageQuery.isError ? <ErrorState message={t("skills.assessment.loadError")} onRetry={() => void pageQuery.refetch()} /> : null}
    {pageQuery.data?.items.length === 0 ? <div className="mt-4 rounded-lg border border-dashed border-border p-5 text-center"><ShieldCheck className="mx-auto h-5 w-5 text-muted-foreground" /><p className="mt-2 text-sm font-medium">{t("skills.assessment.empty")}</p><p className="mt-1 text-xs leading-5 text-muted-foreground">{t("skills.assessment.emptyHint")}</p></div> : null}

    {current?.status === "pending" || current?.status === "running" ? <Status text={t("skills.assessment.processing")} /> : null}
    {current?.status === "failed" ? <ErrorState message={t("skills.assessment.failed")} onRetry={() => reassess.mutate()} /> : null}
    {activeAttemptId && detailQuery.isLoading ? <Status text={t("skills.assessment.loadingDetail")} /> : null}
    {detailQuery.isError ? <ErrorState message={t("skills.assessment.detailError")} onRetry={() => void detailQuery.refetch()} /> : null}
    {detailQuery.data ? <AssessmentExplanation detail={detailQuery.data} /> : null}

    {pageQuery.data && pageQuery.data.items.length > 0 ? <div className="mt-5 grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(18rem,0.65fr)]">
      <AssessmentPolicy
        confirmed={disclosureConfirmed}
        error={consent.isError}
        onConfirmed={setDisclosureConfirmed}
        onToggle={(enabled) => consent.mutate(enabled)}
        policy={policyQuery.data}
        saving={consent.isPending}
      />
      <AssessmentHistory items={pageQuery.data.items} onSelect={setSelectedAttemptId} selectedId={activeAttemptId} />
    </div> : null}

    {current ? <div className="mt-4 flex flex-wrap items-center justify-between gap-3 border-t border-border pt-4">
      <p className="max-w-2xl text-xs leading-5 text-muted-foreground">{t("skills.assessment.reassessHint")}</p>
      <Button disabled={reassess.isPending} onClick={() => reassess.mutate()} size="sm" variant="outline"><RefreshCw />{reassess.isPending ? t("skills.assessment.scheduling") : t("skills.assessment.reassess")}</Button>
      {reassess.data ? <p className="w-full text-right text-xs text-muted-foreground" role="status">{t(`skills.assessment.schedule.${reassess.data.status}`)}</p> : null}
      {reassess.isError ? <p className="w-full text-right text-xs text-destructive" role="alert">{t("skills.assessment.reassessError")}</p> : null}
    </div> : null}
  </section>;
}

function AssessmentPolicy({ confirmed, error, onConfirmed, onToggle, policy, saving }: {
  confirmed: boolean;
  error: boolean;
  onConfirmed: (value: boolean) => void;
  onToggle: (enabled: boolean) => void;
  policy?: Awaited<ReturnType<typeof agentService.getSkillEvolutionAssessmentPolicy>>;
  saving: boolean;
}) {
  const { t } = useTranslation();
  return <section aria-labelledby="assessment-policy-heading" className="rounded-lg border border-border bg-background p-3">
    <div className="flex flex-wrap items-center justify-between gap-2"><h5 className="text-xs font-semibold" id="assessment-policy-heading">{t("skills.assessment.policyTitle")}</h5>{policy ? <Badge tone={policy.modelEvaluationEnabled ? "success" : "muted"}>{t(policy.modelEvaluationEnabled ? "skills.assessment.enabled" : "skills.assessment.disabled")}</Badge> : null}</div>
    {policy ? <><p className="mt-2 text-xs leading-5 text-muted-foreground">{t("skills.assessment.disclosure")}</p><p className="mt-2 text-xs leading-5 text-muted-foreground">{t(policy.providerAvailable ? "skills.assessment.providerAvailable" : "skills.assessment.providerUnavailable")}</p>
      {!policy.modelEvaluationEnabled ? <label className="mt-3 flex cursor-pointer items-start gap-2 rounded-md bg-muted/40 p-2 text-xs leading-5"><input checked={confirmed} className="mt-1 h-4 w-4 accent-primary" onChange={(event) => onConfirmed(event.target.checked)} type="checkbox" /><span>{t("skills.assessment.confirmDisclosure", { version: policy.disclosureVersion })}</span></label> : null}
      <Button className="mt-3" disabled={saving || (!policy.modelEvaluationEnabled && (!confirmed || !policy.providerAvailable))} onClick={() => onToggle(!policy.modelEvaluationEnabled)} size="sm" variant="outline">{saving ? t("skills.assessment.saving") : t(policy.modelEvaluationEnabled ? "skills.assessment.disable" : "skills.assessment.enable")}</Button>
      {error ? <p className="mt-2 text-xs text-destructive" role="alert">{t("skills.assessment.consentError")}</p> : null}</> : <Status text={t("skills.assessment.loadingPolicy")} />}
  </section>;
}

function Status({ text }: { text: string }) { return <p className="mt-4 rounded-md border border-border bg-muted/30 p-3 text-xs text-muted-foreground" role="status">{text}</p>; }
function ErrorState({ message, onRetry }: { message: string; onRetry: () => void }) { const { t } = useTranslation(); return <div className="mt-4 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-xs text-destructive" role="alert"><p>{message}</p><Button className="mt-2" onClick={onRetry} size="sm" variant="outline">{t("skills.assessment.retry")}</Button></div>; }
function statusTone(status: string) { return status === "completed" ? "success" as const : status === "failed" ? "danger" as const : status === "superseded" ? "muted" as const : "warning" as const; }
