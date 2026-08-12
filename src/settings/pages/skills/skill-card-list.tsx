import { Eye, Link2, LoaderCircle, PanelRightOpen, Pencil, Trash2, Unlink2 } from "lucide-react";
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
import { SkillRowSummary } from "./skill-row-summary";

interface SkillCardListProps {
  skills: Skill[];
  activeAgent: SkillCompatibleAgent | null;
  apiBindingsBySkillId: Record<string, string[]>;
  busySkillId: string | null;
  bindingSkillId: string | null;
  operationError: string | null;
  operationSkillId: string | null;
  selectedSkillKey: string | null;
  filtered: boolean;
  onToggleEnabled: (skill: Skill, enabled: boolean) => void;
  onToggleAgent: (skill: Skill, assigned: boolean) => void;
  onDetails: (skill: Skill, trigger: HTMLButtonElement) => void;
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
  return <div
    aria-label={t("skills.assignment.boardAriaLabel", { agent: props.activeAgent.displayName })}
    className="grid items-start gap-4 xl:grid-cols-2"
    data-testid="skill-selection-board"
    role="group"
  >
    <SkillSelectionPanel {...props} activeAgent={props.activeAgent} group="assigned" skills={assigned} />
    <SkillSelectionPanel {...props} activeAgent={props.activeAgent} group="available" skills={available} />
  </div>;
}

function SkillSelectionPanel({ group, skills, ...props }: SkillCardListProps & {
  activeAgent: SkillCompatibleAgent;
  group: "assigned" | "available";
}) {
  const { t } = useTranslation();
  const headingId = `skill-selection-${props.activeAgent.id}-${group}`;
  return <section aria-labelledby={headingId} className="ucd-panel min-w-0 overflow-hidden rounded-lg" data-skill-group={group}>
    <div className="flex items-start justify-between gap-3 border-b border-border px-4 py-3">
      <div className="min-w-0">
        <h3 className="text-sm font-semibold" id={headingId}>{t(`skills.assignment.${group}`)}</h3>
        <p className="mt-1 text-xs leading-5 text-muted-foreground">{t(`skills.assignment.${group}Description`)}</p>
      </div>
      <Badge tone={group === "assigned" ? "default" : "muted"}>{skills.length}</Badge>
    </div>
    {skills.length > 0
      ? <AgentSkillRows {...props} skills={skills} />
      : <p className="p-4 text-xs leading-5 text-muted-foreground">{t(`skills.assignment.${group}Empty`)}</p>}
  </section>;
}

function AgentSkillRows(props: SkillCardListProps & { activeAgent: SkillCompatibleAgent }) {
  const { t } = useTranslation();
  return <div>{props.skills.map((skill) => {
    const assigned = isSkillAssignedToAgent(skill, props.activeAgent, props.apiBindingsBySkillId);
    const bindingState = getSkillBindingState(skill, props.activeAgent, props.apiBindingsBySkillId);
    const selected = props.selectedSkillKey === skillIdentity(skill);
    const busy = props.busySkillId === skill.id;
    const bindingPending = props.bindingSkillId === skill.id;
    const assignmentUnavailable = !assigned && (skill.availability !== "available" || skill.metadata.type === "utility");
    return <article
      className={rowClassName(selected)}
      data-selected={selected}
      data-skill-id={skill.id}
      key={skillIdentity(skill)}
    >
      <SkillRowSummary agent={props.activeAgent} bindingState={bindingState} selected={selected} skill={skill} />
      <div className="mt-3 flex flex-wrap items-center justify-end gap-2">
        <DetailsButton onDetails={props.onDetails} selected={selected} skill={skill} />
        <Button aria-label={t("skills.preview")} className="h-11 w-11 sm:h-9 sm:w-9" disabled={busy} onClick={() => props.onPreview(skill)} size="icon" variant="outline"><Eye /></Button>
        {!assignmentUnavailable ? <Button
          aria-label={t(assigned ? "skills.assignment.removeFrom" : "skills.assignment.addTo", { agent: props.activeAgent.displayName })}
          disabled={busy}
          onClick={() => props.onToggleAgent(skill, !assigned)}
          className="min-h-11 sm:min-h-8"
          size="sm"
          variant={assigned ? "outline" : "default"}
        >
          {bindingPending ? <LoaderCircle className="animate-spin motion-reduce:animate-none" /> : assigned ? <Unlink2 /> : <Link2 />}
          {t(bindingPending
            ? assigned ? "skills.assignment.removing" : "skills.assignment.assigning"
            : assigned ? "skills.assignment.remove" : "skills.assignment.add")}
        </Button> : null}
        {props.operationSkillId === skill.id && props.operationError
          ? <p className="basis-full text-xs leading-5 text-destructive" role="alert">{props.operationError}</p>
          : null}
      </div>
    </article>;
  })}</div>;
}

function LifecycleSkillRows(props: SkillCardListProps) {
  const { t } = useTranslation();
  return <div className="ucd-panel overflow-hidden rounded-lg">{props.skills.map((skill) => {
    const selected = props.selectedSkillKey === skillIdentity(skill);
    const busy = props.busySkillId === skill.id;
    return <article
      className={`${rowClassName(selected)} md:grid md:grid-cols-[minmax(0,1fr)_auto] md:items-start`}
      data-selected={selected}
      data-skill-id={skill.id}
      key={skillIdentity(skill)}
    >
      <SkillRowSummary selected={selected} skill={skill} />
      <div className="flex flex-wrap items-center justify-end gap-2">
        <label className="flex min-h-11 items-center gap-2 px-1 text-xs"><input aria-label={t("skills.enabled")} checked={skill.enabled} disabled={busy} onChange={(event) => props.onToggleEnabled(skill, event.target.checked)} type="checkbox" />{t("skills.enabled")}</label>
        <DetailsButton onDetails={props.onDetails} selected={selected} skill={skill} />
        <Button aria-label={t("skills.preview")} className="h-11 w-11 sm:h-9 sm:w-9" disabled={busy} onClick={() => props.onPreview(skill)} size="icon" variant="outline"><Eye /></Button>
        {!skill.immutable ? <Button aria-label={t("skills.edit")} className="h-11 w-11 sm:h-9 sm:w-9" disabled={busy} onClick={() => props.onEdit(skill)} size="icon" variant="outline"><Pencil /></Button> : null}
        {!skill.immutable ? <Button aria-label={t("skills.delete")} className="h-11 w-11 sm:h-9 sm:w-9" disabled={busy} onClick={() => props.onDelete(skill)} size="icon" variant="ghost"><Trash2 /></Button> : null}
        {props.operationSkillId === skill.id && props.operationError
          ? <p className="basis-full text-xs leading-5 text-destructive" role="alert">{props.operationError}</p>
          : null}
      </div>
    </article>;
  })}</div>;
}

function DetailsButton({ skill, selected, onDetails }: {
  skill: Skill;
  selected: boolean;
  onDetails: SkillCardListProps["onDetails"];
}) {
  const { t } = useTranslation();
  return <Button
    aria-expanded={selected}
    aria-label={t("skills.details.open", { name: skill.metadata.name })}
    className="h-11 w-11 sm:h-9 sm:w-9"
    onClick={(event) => onDetails(skill, event.currentTarget)}
    size="icon"
    variant="outline"
  ><PanelRightOpen /></Button>;
}

function rowClassName(selected: boolean) {
  return `grid min-w-0 gap-3 border-b border-border p-3 transition-colors last:border-b-0 motion-reduce:transition-none ${selected ? "bg-accent/50 ring-1 ring-inset ring-primary/40" : "bg-transparent"}`;
}
