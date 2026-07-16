import { Eye, Pencil, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
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
    <div className="grid gap-4 xl:grid-cols-2">
      {skills.map((skill) => (
        <section className="ucd-panel rounded-lg p-4" key={`${skill.scope}:${skill.workspacePath ?? ""}:${skill.id}`}>
          <div className="mb-3 flex items-start justify-between gap-3">
            <div className="min-w-0">
              <h3 className="truncate font-semibold">{skill.metadata.name}</h3>
              <p className="mt-1 text-xs text-muted-foreground">{skill.id}</p>
            </div>
            <span className="rounded-sm bg-[hsl(var(--nav-active-soft))] px-2 py-1 text-xs font-medium text-primary">
              {t(`skills.source.${skill.source}`)}
            </span>
          </div>
          <p className="min-h-10 text-sm text-muted-foreground">{skill.metadata.description}</p>
          <div className="mt-3 flex flex-wrap gap-1">
            <span className="rounded-sm bg-muted px-2 py-1 text-xs">{skill.metadata.category}</span>
            {skill.metadata.triggers.slice(0, 3).map((trigger) => (
              <span className="rounded-sm bg-muted px-2 py-1 text-xs" key={trigger}>
                {trigger}
              </span>
            ))}
          </div>
          <div className="mt-4 flex items-center justify-between border-t border-border pt-3">
            <label className="flex items-center gap-2 text-sm">
              <input
                checked={skill.enabled}
                disabled={busySkillId === skill.id}
                onChange={(event) => onToggleEnabled(skill, event.target.checked)}
                type="checkbox"
              />
              {t("skills.enabled")}
            </label>
            <div className="flex gap-2">
              <Button onClick={() => onPreview(skill)} variant="outline">
                <Eye className="h-4 w-4" aria-hidden="true" />
              </Button>
              <Button onClick={() => onEdit(skill)} variant="outline">
                <Pencil className="h-4 w-4" aria-hidden="true" />
              </Button>
              <Button onClick={() => onDelete(skill)} variant="ghost">
                <Trash2 className="h-4 w-4" aria-hidden="true" />
              </Button>
            </div>
          </div>
          <div className="mt-4 grid grid-flow-col auto-cols-[minmax(7rem,1fr)] gap-2 overflow-x-auto pb-1">
            {agents.map((agent) => (
              <label
                className="flex min-w-0 items-center gap-2 rounded-md border border-border px-2 py-2 text-sm"
                key={agent.id}
              >
                <input
                  className="shrink-0"
                  checked={skill.boundAgentIds.includes(agent.id)}
                  onChange={(event) => onToggleAgent(skill, agent.id, event.target.checked)}
                  type="checkbox"
                />
                <span className="min-w-0 flex-1">
                  <span className="block truncate">{agent.displayName}</span>
                </span>
              </label>
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}
