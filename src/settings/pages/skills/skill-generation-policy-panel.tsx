import { useMutation, useQuery } from "@tanstack/react-query";
import { LockKeyhole, ShieldCheck } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { formatAppNumber } from "../../../i18n/format";
import type { SkillGenerationService } from "../../../services/skill-generation-service";

export function SkillGenerationPolicyPanel({ service, workspaceId }: { service: SkillGenerationService; workspaceId: string }) {
  const { i18n, t } = useTranslation();
  const policy = useQuery({ queryKey: ["skill-generation-policy", workspaceId], queryFn: () => service.getGenerationPolicy(workspaceId) });
  const [profileId, setProfileId] = useState("");
  const [modelId, setModelId] = useState("");
  useEffect(() => { setProfileId(policy.data?.providerProfileId ?? ""); setModelId(policy.data?.modelId ?? ""); }, [policy.data?.modelId, policy.data?.providerProfileId]);
  const update = useMutation({
    mutationFn: (enabled: boolean) => service.updateGenerationPolicy({
      workspaceId, expectedRevision: policy.data?.revision ?? 0, enabled,
      disclosureVersion: policy.data?.disclosureVersion ?? "generation-disclosure-v1",
      providerProfileId: profileId.trim() || undefined, modelId: modelId.trim() || undefined,
      allowedArtifactKinds: policy.data?.allowedArtifactKinds ?? ["overlay_learn_block", "overlay_exact_patch", "new_skill"],
    }),
    onSuccess: () => void policy.refetch(),
  });
  if (policy.isLoading) return <p className="rounded-xl border border-border p-4 text-sm text-muted-foreground" role="status">{t("skills.generation.policyLoading")}</p>;
  if (!policy.data) return <p className="rounded-xl border border-destructive/40 bg-destructive/10 p-4 text-sm text-destructive" role="alert">{policy.error?.message ?? t("skills.generation.policyError")}</p>;
  const value = policy.data;
  return <section aria-labelledby="generation-policy-title" className="rounded-xl border border-border bg-background p-4 shadow-sm"><div className="flex flex-wrap items-start justify-between gap-3"><div className="flex items-start gap-2"><ShieldCheck className="mt-0.5 h-4 w-4 text-primary" /><div><h3 className="text-sm font-semibold" id="generation-policy-title">{t("skills.generation.policyTitle")}</h3><p className="mt-1 max-w-3xl text-xs leading-5 text-muted-foreground">{t("skills.generation.disclosure")}</p></div></div><Badge tone={value.enabled ? "success" : "muted"}>{t(value.enabled ? "skills.generation.enabled" : "skills.generation.disabled")}</Badge></div>
    <div className="mt-4 grid gap-3 sm:grid-cols-2"><Field label={t("skills.generation.providerProfile")} onChange={setProfileId} value={profileId} /><Field label={t("skills.generation.model")} onChange={setModelId} value={modelId} /></div>
    <dl className="mt-4 grid grid-cols-2 gap-2 text-xs sm:grid-cols-4"><Metric label={t("skills.generation.dailyInput")} value={formatAppNumber(value.dailyInputTokens, i18n.language)} /><Metric label={t("skills.generation.dailyOutput")} value={formatAppNumber(value.dailyOutputTokens, i18n.language)} /><Metric label={t("skills.generation.failedRetention")} value={`${value.failedCancelledRetentionDays}d`} /><Metric label={t("skills.generation.packageRetention")} value={`${value.completedPackageRetentionDays}d`} /></dl>
    <div className="mt-4 flex flex-wrap items-center justify-between gap-3 border-t border-border pt-3"><span className="inline-flex items-center gap-1.5 text-xs text-muted-foreground"><LockKeyhole className="h-3.5 w-3.5" />{t("skills.generation.autoApplyExcluded")}</span><Button disabled={update.isPending} onClick={() => update.mutate(!value.enabled)} variant={value.enabled ? "outline" : "default"}>{t(value.enabled ? "skills.generation.revoke" : "skills.generation.enable")}</Button></div>
    {update.isError ? <p className="mt-3 text-xs text-destructive" role="alert">{update.error.message}</p> : null}
  </section>;
}

function Field({ label, onChange, value }: { label: string; onChange: (value: string) => void; value: string }) {
  return <label className="text-xs text-muted-foreground"><span>{label}</span><input className="mt-1 h-9 w-full rounded-md border border-border bg-muted/20 px-3 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => onChange(event.target.value)} value={value} /></label>;
}
function Metric({ label, value }: { label: string; value: string }) { return <div className="rounded-lg bg-muted/40 p-2.5"><dt className="text-muted-foreground">{label}</dt><dd className="mt-1 font-medium tabular-nums text-foreground">{value}</dd></div>; }
