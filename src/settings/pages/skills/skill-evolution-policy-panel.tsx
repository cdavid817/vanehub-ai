import { ShieldCheck } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type {
  EvolutionPolicy,
  EvolutionPolicyMode,
  EvolutionPolicyUpdate,
} from "../../../services/skill-evolution-orchestration-service";

interface Props {
  error: string | null;
  onSave: (input: EvolutionPolicyUpdate) => void;
  policy: EvolutionPolicy;
  saving: boolean;
}

export function SkillEvolutionPolicyPanel({ error, onSave, policy, saving }: Props) {
  const { t } = useTranslation();
  const [mode, setMode] = useState<EvolutionPolicyMode>(policy.mode);
  const [allowlist, setAllowlist] = useState(policy.allowedSkillIds.join("\n"));
  const [acknowledged, setAcknowledged] = useState(false);
  useEffect(() => {
    setMode(policy.mode);
    setAllowlist(policy.allowedSkillIds.join("\n"));
    setAcknowledged(false);
  }, [policy]);
  const ids = normalizedSkillIds(allowlist);
  const enabledBlocked = mode === "enabled" && (!ids.length || (!policy.consent && !acknowledged));
  return <section aria-labelledby="evolution-policy-title" className="rounded-xl border border-border bg-background p-4 shadow-sm">
    <div className="flex flex-wrap items-start justify-between gap-3">
      <div><div className="flex items-center gap-2"><ShieldCheck className="h-4 w-4 text-primary" /><h3 className="font-semibold" id="evolution-policy-title">{t("skills.evolution.policy.title")}</h3></div><p className="mt-1 max-w-3xl text-xs leading-5 text-muted-foreground">{t("skills.evolution.policy.description")}</p></div>
      <Badge tone={policy.mockProvenance ? "warning" : "success"}>{t(policy.mockProvenance ? "skills.evolution.capability.web" : "skills.evolution.capability.desktop")}</Badge>
    </div>
    <div aria-label={t("skills.evolution.policy.modeLabel")} className="mt-4 grid gap-2 sm:grid-cols-3" role="radiogroup">
      {(["off", "observe", "enabled"] as const).map((item) => <button aria-checked={mode === item} className={`rounded-lg border p-3 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${mode === item ? "border-primary bg-primary/10" : "border-border hover:bg-muted/40"}`} key={item} onClick={() => setMode(item)} role="radio" type="button"><span className="block text-sm font-medium">{t(`skills.evolution.mode.${item}`)}</span><span className="mt-1 block text-xs leading-5 text-muted-foreground">{t(`skills.evolution.mode.${item}Description`)}</span></button>)}
    </div>
    <label className="mt-4 block text-xs font-medium text-muted-foreground"><span>{t("skills.evolution.policy.allowlist")}</span><textarea aria-describedby="evolution-allowlist-help" className="mt-1 min-h-24 w-full resize-y rounded-lg border border-border bg-background p-3 font-mono text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => setAllowlist(event.target.value)} placeholder={t("skills.evolution.policy.allowlistPlaceholder")} value={allowlist} /></label>
    <p className="mt-1 text-xs text-muted-foreground" id="evolution-allowlist-help">{t("skills.evolution.policy.allowlistHelp")}</p>
    <div className="mt-4 rounded-lg border border-amber-500/30 bg-amber-500/10 p-3 text-xs leading-5">
      <p className="font-medium">{t("skills.evolution.policy.disclosureTitle")}</p><p className="mt-1 text-muted-foreground">{t("skills.evolution.policy.disclosure")}</p>
      <label className="mt-3 flex items-start gap-2"><input checked={acknowledged} className="mt-0.5 h-4 w-4 accent-primary" onChange={(event) => setAcknowledged(event.target.checked)} type="checkbox" /><span>{t(policy.consent ? "skills.evolution.policy.reacknowledge" : "skills.evolution.policy.acknowledge")}</span></label>
    </div>
    <dl className="mt-4 grid gap-2 text-xs sm:grid-cols-3"><PolicyFact label={t("skills.evolution.policy.exclusions")} value={t("skills.evolution.policy.exclusionsValue")} /><PolicyFact label={t("skills.evolution.policy.rate") } value={t("skills.evolution.policy.rateValue")} /><PolicyFact label={t("skills.evolution.policy.cooldown")} value={t("skills.evolution.policy.cooldownValue")} /></dl>
    {error ? <p className="mt-3 text-sm text-destructive" role="alert">{error}</p> : null}
    {enabledBlocked ? <p className="mt-3 text-xs text-amber-700 dark:text-amber-300" role="status">{t("skills.evolution.policy.enabledRequirements")}</p> : null}
    <div className="mt-4 flex justify-end"><Button disabled={saving || enabledBlocked} onClick={() => onSave({ workspaceId: policy.workspaceId, expectedRevision: policy.revision, mode, allowedSkillIds: ids, acknowledgeCurrentDisclosure: acknowledged })}>{saving ? t("skills.evolution.policy.saving") : t("skills.evolution.policy.save")}</Button></div>
  </section>;
}

function normalizedSkillIds(value: string) {
  return [...new Set(value.split(/[\n,]/).map((item) => item.trim()).filter(Boolean))].sort();
}

function PolicyFact({ label, value }: { label: string; value: string }) {
  return <div className="rounded-lg border border-border bg-muted/20 p-3"><dt className="font-medium">{label}</dt><dd className="mt-1 text-muted-foreground">{value}</dd></div>;
}
