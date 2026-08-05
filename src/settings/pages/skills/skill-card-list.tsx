import { Eye, Link2, LoaderCircle, Pencil, Trash2, Unlink2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import {
  getSkillBindingState,
  isSkillAssignedToAgent,
  partitionSkillsForAgent,
  skillIdentity,
} from "../../../lib/skill-management";
import type { Skill, SkillCompatibleAgent } from "../../../types/skill";

interface SkillCardListProps {
  skills: Skill[];
  activeAgent: SkillCompatibleAgent | null;
  apiBindingsBySkillId: Record<string, string[]>;
  busySkillId: string | null;
  bindingSkillId: string | null;
  operationError: string | null;
  operationSkillId: string | null;
  filtered: boolean;
  onToggleEnabled: (skill: Skill, enabled: boolean) => void;
  onToggleAgent: (skill: Skill, assigned: boolean) => void;
  onPreview: (skill: Skill) => void;
  onEdit: (skill: Skill) => void;
  onDelete: (skill: Skill) => void;
}

export function SkillCardList(props: SkillCardListProps) {
  const { t } = useTranslation();
  if (props.skills.length === 0) {
    return <div className="ucd-panel rounded-lg p-6 text-sm text-muted-foreground">{t(props.filtered ? "skills.empty.filtered" : "skills.empty.inventory")}</div>;
  }
  if (!props.activeAgent) return <LifecycleSkillRows {...props} />;

  const { assigned, available } = partitionSkillsForAgent(
    props.skills,
    props.activeAgent,
    props.apiBindingsBySkillId,
  );
  return (
    <div
      aria-label={t("skills.assignment.boardAriaLabel", { agent: props.activeAgent.displayName })}
      className="grid items-start gap-4 xl:grid-cols-2"
      data-testid="skill-selection-board"
      role="group"
    >
      <SkillSelectionPanel {...props} activeAgent={props.activeAgent} group="assigned" skills={assigned} />
      <SkillSelectionPanel {...props} activeAgent={props.activeAgent} group="available" skills={available} />
    </div>
  );
}

function SkillSelectionPanel({ group, skills, ...props }: SkillCardListProps & {
  activeAgent: SkillCompatibleAgent;
  group: "assigned" | "available";
}) {
  const { t } = useTranslation();
  const headingId = `skill-selection-${props.activeAgent.id}-${group}`;
  return (
    <section
      aria-labelledby={headingId}
      className="ucd-panel min-w-0 overflow-hidden rounded-lg"
      data-skill-group={group}
    >
      <div className="flex items-start justify-between gap-3 border-b border-border px-4 py-3">
        <div className="min-w-0">
          <h3 className="text-sm font-semibold" id={headingId}>{t(`skills.assignment.${group}`)}</h3>
          <p className="mt-1 text-xs leading-5 text-muted-foreground">{t(`skills.assignment.${group}Description`)}</p>
        </div>
        <Badge tone={group === "assigned" ? "default" : "muted"}>{skills.length}</Badge>
      </div>
      {skills.length > 0
        ? <AgentSkillRows {...props} group={group} skills={skills} />
        : <p className="p-4 text-xs leading-5 text-muted-foreground">{t(`skills.assignment.${group}Empty`)}</p>}
    </section>
  );
}

function AgentSkillRows({
  skills,
  activeAgent,
  apiBindingsBySkillId,
  busySkillId,
  bindingSkillId,
  operationError,
  operationSkillId,
  onToggleAgent,
  onPreview,
}: SkillCardListProps & { activeAgent: SkillCompatibleAgent; group: "assigned" | "available" }) {
  const { t } = useTranslation();
  return <div>{skills.map((skill) => {
    const assigned = isSkillAssignedToAgent(skill, activeAgent, apiBindingsBySkillId);
    const bindingState = getSkillBindingState(skill, activeAgent, apiBindingsBySkillId);
    const busy = busySkillId === skill.id;
    const bindingPending = bindingSkillId === skill.id;
    return (
      <article className="border-b border-border p-3 last:border-b-0" data-skill-id={skill.id} key={skillIdentity(skill)}>
        <SkillSummary agent={activeAgent} bindingState={bindingState} skill={skill} />
        <div className="mt-3 flex flex-wrap items-center justify-end gap-2">
          <Button aria-label={t("skills.preview")} disabled={busy} onClick={() => onPreview(skill)} size="icon" variant="outline"><Eye /></Button>
          <Button
            aria-label={t(assigned ? "skills.assignment.removeFrom" : "skills.assignment.addTo", { agent: activeAgent.displayName })}
            disabled={busy}
            onClick={() => onToggleAgent(skill, !assigned)}
            size="sm"
            variant={assigned ? "outline" : "default"}
          >
            {bindingPending ? <LoaderCircle className="animate-spin" /> : assigned ? <Unlink2 /> : <Link2 />}
            {t(bindingPending
              ? assigned ? "skills.assignment.removing" : "skills.assignment.assigning"
              : assigned ? "skills.assignment.remove" : "skills.assignment.add")}
          </Button>
          {operationSkillId === skill.id && operationError
            ? <p className="basis-full text-xs leading-5 text-destructive" role="alert">{operationError}</p>
            : null}
        </div>
      </article>
    );
  })}</div>;
}

function LifecycleSkillRows(props: SkillCardListProps) {
  const { t } = useTranslation();
  return <div className="ucd-panel overflow-hidden rounded-lg">{props.skills.map((skill) => {
    const busy = props.busySkillId === skill.id;
    return (
      <article className="grid gap-3 border-b border-border p-3 last:border-b-0 md:grid-cols-[minmax(0,1fr)_auto] md:items-center" key={skillIdentity(skill)}>
        <SkillSummary skill={skill} />
        <div className="flex flex-wrap items-center gap-2">
          <label className="flex h-9 items-center gap-2 px-1 text-xs"><input aria-label={t("skills.enabled")} checked={skill.enabled} disabled={busy} onChange={(event) => props.onToggleEnabled(skill, event.target.checked)} type="checkbox" />{t("skills.enabled")}</label>
          <Button aria-label={t("skills.preview")} disabled={busy} onClick={() => props.onPreview(skill)} size="icon" variant="outline"><Eye /></Button>
          <Button aria-label={t("skills.edit")} disabled={busy} onClick={() => props.onEdit(skill)} size="icon" variant="outline"><Pencil /></Button>
          <Button aria-label={t("skills.delete")} onClick={() => props.onDelete(skill)} size="icon" variant="ghost"><Trash2 /></Button>
          {props.operationSkillId === skill.id && props.operationError
            ? <p className="basis-full text-xs leading-5 text-destructive" role="alert">{props.operationError}</p>
            : null}
        </div>
      </article>
    );
  })}</div>;
}

function SkillSummary({ skill, agent, bindingState }: {
  skill: Skill;
  agent?: SkillCompatibleAgent;
  bindingState?: ReturnType<typeof getSkillBindingState>;
}) {
  const { t } = useTranslation();
  return (
    <div className="min-w-0">
      <div className="flex flex-wrap items-center gap-2">
        <h4 className="min-w-0 truncate text-sm font-semibold" title={skill.metadata.name}>{skill.metadata.name}</h4>
        <Badge tone={skill.enabled ? "success" : "muted"}>{agent ? t(skill.enabled ? "skills.globalStatus.enabled" : "skills.globalStatus.paused") : skill.enabled ? t("skills.enabled") : t("basic.disabled")}</Badge>
        <Badge tone="muted">{t(`skills.source.${skill.source}`)}</Badge>
        <Badge tone="muted">v{skill.metadata.version}</Badge>
        {bindingState ? <Badge tone={bindingState === "available" ? "muted" : "default"}>{t(`skills.binding.${bindingState}`)}</Badge> : null}
      </div>
      <p className="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">{skill.metadata.description}</p>
      <p className="mt-1 truncate font-mono text-[11px] text-muted-foreground">{skill.id} · {skill.metadata.category}</p>
    </div>
  );
}
