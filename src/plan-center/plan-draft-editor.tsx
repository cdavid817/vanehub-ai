import { ArrowDown, ArrowUp, Plus, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { PlanDependency, PlanDraft, PlanSubTask } from "../types/plan";
import { PlanPolicyEditor, TaskVerificationEditor } from "./plan-policy-editor";

const inputClass = "ucd-input w-full rounded-md px-3 py-2 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

export function validatePlanDraftReview(draft: PlanDraft): string | null {
  if (draft.subtasks.length < 1 || draft.subtasks.length > 10) return "plans.validation.taskCount";
  const ids = new Set(draft.subtasks.map((task) => task.id));
  if (ids.size !== draft.subtasks.length) return "plans.validation.duplicateTask";
  if (draft.subtasks.some((task) => !task.title.trim() || !task.description.trim())) return "plans.validation.taskFields";
  if (draft.subtasks.some((task) => task.acceptanceCriteria.length < 1 || task.acceptanceCriteria.length > 3)) return "plans.validation.criteria";
  if (draft.subtasks.some((task) => !task.validationCommands.some((command) => command.required))) return "plans.validation.commands";
  if (draft.subtasks.some((task) => task.criterionEvidence.length !== task.acceptanceCriteria.length)) return "plans.validation.evidence";
  if (draft.executionPolicy.maxAttemptsPerSubtask < 1 || draft.executionPolicy.maxAttemptsPerSubtask > 5) return "plans.validation.attempts";
  if (!draft.executionPolicy.finalValidationCommands.some((command) => command.required)) return "plans.validation.finalCommands";
  const successors = new Map<string, string[]>();
  const edges = new Set<string>();
  for (const edge of draft.dependencies) {
    if (!ids.has(edge.predecessorId) || !ids.has(edge.successorId) || edge.predecessorId === edge.successorId) return "plans.validation.edge";
    const identity = `${edge.predecessorId}\u0000${edge.successorId}`;
    if (edges.has(identity)) return "plans.validation.edge";
    edges.add(identity);
    successors.set(edge.predecessorId, [...(successors.get(edge.predecessorId) ?? []), edge.successorId]);
  }
  const visiting = new Set<string>();
  const visited = new Set<string>();
  const cyclic = (id: string): boolean => {
    if (visiting.has(id)) return true;
    if (visited.has(id)) return false;
    visiting.add(id);
    const found = (successors.get(id) ?? []).some(cyclic);
    visiting.delete(id);
    visited.add(id);
    return found;
  };
  return [...ids].some(cyclic) ? "plans.validation.cycle" : null;
}

export function PlanDraftEditor({ draft, onChange }: { draft: PlanDraft; onChange: (draft: PlanDraft) => void }) {
  const { t } = useTranslation();
  const updateTask = (id: string, update: Partial<PlanSubTask>) => onChange({ ...draft, subtasks: draft.subtasks.map((task) => task.id === id ? { ...task, ...update } : task) });
  const move = (index: number, offset: number) => {
    const tasks = [...draft.subtasks];
    const target = index + offset;
    if (target < 0 || target >= tasks.length) return;
    [tasks[index], tasks[target]] = [tasks[target]!, tasks[index]!];
    onChange({ ...draft, subtasks: tasks.map((task, ordinal) => ({ ...task, ordinal })) });
  };
  const removeTask = (id: string) => onChange({
    ...draft,
    subtasks: draft.subtasks.filter((task) => task.id !== id).map((task, ordinal) => ({ ...task, ordinal })),
    dependencies: draft.dependencies.filter((edge) => edge.predecessorId !== id && edge.successorId !== id),
  });
  const addTask = () => {
    const id = `task-${crypto.randomUUID()}`;
    onChange({ ...draft, subtasks: [...draft.subtasks, {
      id, title: t("plans.review.newTask"), description: "", acceptanceCriteria: [""],
      criterionEvidence: [{ criterionIndex: 0, kind: "manual", commandId: null }],
      ordinal: draft.subtasks.length, assignedRole: "worker",
      limits: { tokenBudget: 8_000, toolCallLimit: 30, timeoutSeconds: 900 }, validationCommands: [{ id: `verify-${id}`, program: "npm", args: ["run", "test"], workingDirectory: null, timeoutSeconds: 600, required: true }],
    }] });
  };

  return (
    <div className="grid gap-4">
      <div className="flex items-center justify-between gap-3">
        <div><h2 className="text-sm font-semibold">{t("plans.review.title")}</h2><p className="text-xs text-muted-foreground">{t("plans.review.description")}</p></div>
        <Button onClick={addTask} size="sm" type="button" variant="outline"><Plus aria-hidden="true" />{t("plans.review.addTask")}</Button>
      </div>
      <ol className="grid gap-3">
        {draft.subtasks.map((task, index) => (
          <li className="ucd-card grid gap-3 rounded-lg p-3" key={task.id}>
            <div className="flex items-center gap-2">
              <span className="grid h-7 w-7 shrink-0 place-items-center rounded-full bg-primary/10 text-xs font-semibold text-primary">{index + 1}</span>
              <input aria-label={t("plans.review.taskTitle", { number: index + 1 })} className={`${inputClass} min-w-0 flex-1 font-medium`} onChange={(event) => updateTask(task.id, { title: event.target.value })} value={task.title} />
              <Button aria-label={t("plans.review.moveUp")} disabled={index === 0} onClick={() => move(index, -1)} size="icon" type="button" variant="ghost"><ArrowUp aria-hidden="true" /></Button>
              <Button aria-label={t("plans.review.moveDown")} disabled={index === draft.subtasks.length - 1} onClick={() => move(index, 1)} size="icon" type="button" variant="ghost"><ArrowDown aria-hidden="true" /></Button>
              <Button aria-label={t("plans.review.removeTask")} onClick={() => removeTask(task.id)} size="icon" type="button" variant="ghost"><Trash2 aria-hidden="true" /></Button>
            </div>
            <label className="grid gap-1 text-xs font-medium text-muted-foreground">{t("plans.review.taskDescription")}<textarea className={`${inputClass} min-h-20`} onChange={(event) => updateTask(task.id, { description: event.target.value })} value={task.description} /></label>
            <label className="grid gap-1 text-xs font-medium text-muted-foreground">{t("plans.review.criteria")}<textarea className={`${inputClass} min-h-20`} onChange={(event) => { const acceptanceCriteria = event.target.value.split("\n").map((value) => value.trim()).filter(Boolean).slice(0, 3); updateTask(task.id, { acceptanceCriteria, criterionEvidence: acceptanceCriteria.map((_, criterionIndex) => task.criterionEvidence.find((binding) => binding.criterionIndex === criterionIndex) ?? { criterionIndex, kind: "manual", commandId: null }) }); }} value={task.acceptanceCriteria.join("\n")} /></label>
            <div className="grid gap-2 sm:grid-cols-3">
              <Limit label={t("plans.review.tokenBudget")} onChange={(value) => updateTask(task.id, { limits: { ...task.limits, tokenBudget: value } })} value={task.limits.tokenBudget} />
              <Limit label={t("plans.review.toolLimit")} onChange={(value) => updateTask(task.id, { limits: { ...task.limits, toolCallLimit: value } })} value={task.limits.toolCallLimit} />
              <Limit label={t("plans.review.timeout")} onChange={(value) => updateTask(task.id, { limits: { ...task.limits, timeoutSeconds: value } })} value={task.limits.timeoutSeconds} />
            </div>
            <TaskVerificationEditor onChange={(update) => updateTask(task.id, update)} task={task} />
          </li>
        ))}
      </ol>
      <DependencyEditor dependencies={draft.dependencies} onChange={(dependencies) => onChange({ ...draft, dependencies })} tasks={draft.subtasks} />
      <PlanPolicyEditor draft={draft} onChange={onChange} />
    </div>
  );
}

function Limit({ label, onChange, value }: { label: string; onChange: (value: number | null) => void; value: number | null }) {
  return <label className="grid gap-1 text-xs font-medium text-muted-foreground">{label}<input className={inputClass} min={1} onChange={(event) => onChange(event.target.value ? Number(event.target.value) : null)} type="number" value={value ?? ""} /></label>;
}

function DependencyEditor({ dependencies, onChange, tasks }: { dependencies: PlanDependency[]; onChange: (edges: PlanDependency[]) => void; tasks: PlanSubTask[] }) {
  const { t } = useTranslation();
  const add = () => tasks.length > 1 && onChange([...dependencies, { predecessorId: tasks[0]!.id, successorId: tasks[1]!.id }]);
  return <fieldset className="ucd-card grid gap-3 rounded-lg p-3"><div className="flex items-center justify-between"><legend className="text-sm font-semibold">{t("plans.review.dependencies")}</legend><Button onClick={add} size="sm" type="button" variant="outline"><Plus aria-hidden="true" />{t("plans.review.addDependency")}</Button></div>{dependencies.map((edge, index) => <div className="grid grid-cols-[1fr_auto_1fr_auto] items-center gap-2" key={`${edge.predecessorId}-${edge.successorId}-${index}`}><TaskSelect onChange={(predecessorId) => onChange(dependencies.map((value, current) => current === index ? { ...value, predecessorId } : value))} tasks={tasks} value={edge.predecessorId} /><span aria-hidden="true" className="text-muted-foreground">→</span><TaskSelect onChange={(successorId) => onChange(dependencies.map((value, current) => current === index ? { ...value, successorId } : value))} tasks={tasks} value={edge.successorId} /><Button aria-label={t("plans.review.removeDependency")} onClick={() => onChange(dependencies.filter((_, current) => current !== index))} size="icon" type="button" variant="ghost"><Trash2 aria-hidden="true" /></Button></div>)}</fieldset>;
}

function TaskSelect({ onChange, tasks, value }: { onChange: (value: string) => void; tasks: PlanSubTask[]; value: string }) {
  return <select className={inputClass} onChange={(event) => onChange(event.target.value)} value={value}>{tasks.map((task) => <option key={task.id} value={task.id}>{task.title}</option>)}</select>;
}
