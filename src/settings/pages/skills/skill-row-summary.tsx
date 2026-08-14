import { CircleCheck, LockKeyhole, TriangleAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import type { Skill, SkillCompatibleAgent } from "../../../types/skill";
import type { SkillBindingState } from "../../../lib/skill-management";

export function SkillRowSummary({ skill, agent, bindingState, selected }: {
  skill: Skill;
  agent?: SkillCompatibleAgent;
  bindingState?: SkillBindingState;
  selected: boolean;
}) {
  const { t } = useTranslation();
  return <div className="min-w-0">
    <div className="flex flex-wrap items-center gap-2">
      <h4 className="min-w-0 truncate text-sm font-semibold" title={skill.metadata.name}>{skill.metadata.name}</h4>
      <Badge tone={skill.enabled ? "success" : "muted"}>{agent ? t(skill.enabled ? "skills.globalStatus.enabled" : "skills.globalStatus.paused") : t(skill.enabled ? "skills.enabled" : "basic.disabled")}</Badge>
      <Badge className="max-w-32 truncate" title={t(`skills.layer.${skill.layer}`)} tone="muted">{t(`skills.layer.${skill.layer}`)}</Badge>
      <Badge className="max-w-32 truncate" title={t(`skills.type.${skill.metadata.type ?? "role"}`)} tone={skill.metadata.type === "utility" ? "warning" : "muted"}>{t(`skills.type.${skill.metadata.type ?? "role"}`)}</Badge>
      {bindingState ? <Badge tone={bindingState === "available" ? "muted" : "default"}>{t(`skills.binding.${bindingState}`)}</Badge> : null}
      {skill.immutable ? <CompactState icon={LockKeyhole} label={t("skills.immutable.compact")} /> : null}
      {skill.metadata.type === "utility" && skill.delegationCapability?.supported ? <CompactState icon={CircleCheck} label={t("skills.utility.compactAvailable")} /> : null}
      {skill.metadata.type === "utility" && !skill.delegationCapability?.supported ? <CompactState icon={TriangleAlert} label={t("skills.utility.compactUnavailable")} warning /> : null}
      {skill.availability !== "available" && !(skill.metadata.type === "utility" && skill.availability === "unsupported") ? <Badge tone="warning">{t(`skills.availability.${skill.availability}`)}</Badge> : null}
      {selected ? <span className="sr-only">{t("skills.details.selected")}</span> : null}
    </div>
    <p className="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">{skill.metadata.description}</p>
    <p className="mt-1 truncate font-mono text-[11px] text-muted-foreground" title={`${skill.id} · ${skill.metadata.category} · v${skill.metadata.version}`}>{skill.id} · {skill.metadata.category} · v{skill.metadata.version}</p>
  </div>;
}

function CompactState({ icon: Icon, label, warning = false }: {
  icon: typeof LockKeyhole;
  label: string;
  warning?: boolean;
}) {
  return <span className={`inline-flex items-center gap-1 text-xs font-medium ${warning ? "text-amber-700 dark:text-amber-300" : "text-muted-foreground"}`}><Icon className="h-3.5 w-3.5" />{label}</span>;
}
