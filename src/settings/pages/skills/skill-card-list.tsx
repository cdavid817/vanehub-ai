import { Eye, Link2, Pencil, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { AgentRegistryEntry } from "../../../types/agent";
import type { Skill } from "../../../types/skill";

export function SkillCardList({
  skills,
  agents,
  busySkillId,
  onToggleEnabled,
  onToggleAgent,
  onPreview,
  onEdit,
  onDelete,
}: {
  skills: Skill[];
  agents: AgentRegistryEntry[];
  busySkillId: string | null;
  onToggleEnabled: (skill: Skill, enabled: boolean) => void;
  onToggleAgent: (skill: Skill, agentId: string, checked: boolean) => void;
  onPreview: (skill: Skill) => void;
  onEdit: (skill: Skill) => void;
  onDelete: (skill: Skill) => void;
}) {
  const { t } = useTranslation();

  if (skills.length === 0) {
    return <div className="ucd-panel rounded-lg p-6 text-sm text-muted-foreground">{t("skills.noMatching")}</div>;
  }

  return (
    <div className="grid items-start gap-4 xl:grid-cols-2">
      {skills.map((skill) => (
        <section className="ucd-panel grid min-h-[18rem] gap-4 rounded-lg p-4" key={`${skill.scope}:${skill.workspacePath ?? ""}:${skill.id}`}>
          <div className="grid gap-3 sm:grid-cols-[minmax(0,1fr)_auto]">
            <div className="min-w-0">
              <div className="flex min-w-0 flex-wrap items-center gap-2">
                <h3 className="min-w-0 truncate text-base font-semibold leading-6">{skill.metadata.name}</h3>
                <Badge tone={skill.enabled ? "success" : "muted"}>{skill.enabled ? t("skills.enabled") : t("basic.disabled")}</Badge>
              </div>
              <p className="mt-1 truncate font-mono text-xs text-muted-foreground" title={skill.id}>{skill.id}</p>
            </div>
            <Badge tone={skill.source === "builtin" ? "default" : "muted"}>{t(`skills.source.${skill.source}`)}</Badge>
          </div>
          <p className="line-clamp-2 min-h-10 text-sm leading-5 text-muted-foreground">{skill.metadata.description}</p>
          <div className="flex flex-wrap gap-1.5">
            <Badge tone="muted">{skill.metadata.category}</Badge>
            {skill.metadata.triggers.slice(0, 3).map((trigger) => (
              <Badge key={trigger} tone="muted">{trigger}</Badge>
            ))}
            {skill.metadata.triggers.length > 3 ? <Badge tone="muted">+{skill.metadata.triggers.length - 3}</Badge> : null}
          </div>
          <div className="mt-auto grid gap-3 border-t border-border pt-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <label className="flex h-9 items-center gap-2 text-sm font-medium">
                <input
                  checked={skill.enabled}
                  className="h-4 w-4 accent-[hsl(var(--primary))]"
                  disabled={busySkillId === skill.id}
                  onChange={(event) => onToggleEnabled(skill, event.target.checked)}
                  type="checkbox"
                />
                {t("skills.enabled")}
              </label>
              <div className="flex gap-2">
                <Button aria-label={t("skills.preview")} onClick={() => onPreview(skill)} size="icon" variant="outline">
                  <Eye className="h-4 w-4" aria-hidden="true" />
                </Button>
                <Button aria-label={t("skills.edit")} onClick={() => onEdit(skill)} size="icon" variant="outline">
                  <Pencil className="h-4 w-4" aria-hidden="true" />
                </Button>
                <Button aria-label={t("skills.delete")} onClick={() => onDelete(skill)} size="icon" variant="ghost">
                  <Trash2 className="h-4 w-4" aria-hidden="true" />
                </Button>
              </div>
            </div>
            <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
              <Link2 className="h-3.5 w-3.5 text-primary" aria-hidden="true" />
              {t("skills.stats.mounted")}
            </div>
            <div className="grid gap-2 sm:grid-cols-2">
              {agents.map((agent) => (
                <label
                  className="flex min-w-0 items-center gap-2 rounded-md border border-border bg-[hsl(var(--panel-muted))] px-2 py-2 text-sm"
                  key={agent.id}
                >
                  <input
                    className="h-4 w-4 shrink-0 accent-[hsl(var(--primary))]"
                    checked={skill.boundAgentIds.includes(agent.id)}
                    onChange={(event) => onToggleAgent(skill, agent.id, event.target.checked)}
                    type="checkbox"
                  />
                  <span className="min-w-0 flex-1 truncate">{agent.displayName}</span>
                </label>
              ))}
            </div>
          </div>
        </section>
      ))}
    </div>
  );
}
