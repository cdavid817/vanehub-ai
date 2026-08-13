import { Plus, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { PlanCriterionEvidenceBinding, PlanDraft, PlanSubTask, PlanVerificationCommand } from "../types/plan";

const fieldClass = "ucd-input w-full rounded-md px-3 py-2 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

function newCommand(prefix: string): PlanVerificationCommand {
  return { id: `${prefix}-${crypto.randomUUID()}`, program: "npm", args: ["run", "test"], workingDirectory: null, timeoutSeconds: 600, required: true };
}

export function PlanPolicyEditor({ draft, onChange }: { draft: PlanDraft; onChange: (draft: PlanDraft) => void }) {
  const { t } = useTranslation();
  const policy = draft.executionPolicy;
  return <section className="ucd-card grid gap-3 rounded-lg p-3" aria-labelledby="plan-policy-title">
    <div><h3 className="text-sm font-semibold" id="plan-policy-title">{t("plans.review.executionPolicy")}</h3><p className="text-xs text-muted-foreground">{t("plans.review.executionPolicyDesc")}</p></div>
    {draft.discovery.status !== "complete" || draft.discovery.limitations.length > 0 ? <div className="rounded-md border border-warning/40 bg-warning/10 p-3 text-xs" role="status"><span className="font-medium">{t("plans.review.discovery", { status: draft.discovery.status })}</span>{draft.discovery.limitations.map((item) => <p className="mt-1 text-muted-foreground" key={item}>{item}</p>)}</div> : null}
    <div className="grid gap-3 sm:grid-cols-2">
      <label className="grid gap-1 text-xs font-medium text-muted-foreground">{t("plans.review.maxAttempts")}<input className={fieldClass} max={5} min={1} onChange={(event) => onChange({ ...draft, executionPolicy: { ...policy, maxAttemptsPerSubtask: Number(event.target.value) } })} type="number" value={policy.maxAttemptsPerSubtask} /></label>
      <label className="grid gap-1 text-xs font-medium text-muted-foreground">{t("plans.review.repairClasses")}<input className={fieldClass} onChange={(event) => onChange({ ...draft, executionPolicy: { ...policy, repairEligibleClasses: event.target.value.split(",").map((item) => item.trim()).filter(Boolean) } })} value={policy.repairEligibleClasses.join(", ")} /></label>
    </div>
    <CommandEditor commands={policy.finalValidationCommands} label={t("plans.review.finalCommands")} onChange={(commands) => onChange({ ...draft, executionPolicy: { ...policy, finalValidationCommands: commands } })} prefix="final" />
  </section>;
}

export function TaskVerificationEditor({ onChange, task }: { onChange: (task: Partial<PlanSubTask>) => void; task: PlanSubTask }) {
  const { t } = useTranslation();
  const updateBinding = (index: number, update: Partial<PlanCriterionEvidenceBinding>) => {
    const bindings = task.acceptanceCriteria.map((_, criterionIndex) => task.criterionEvidence.find((item) => item.criterionIndex === criterionIndex) ?? { criterionIndex, kind: "manual" as const, commandId: null });
    bindings[index] = { ...bindings[index]!, ...update, criterionIndex: index };
    onChange({ criterionEvidence: bindings });
  };
  return <div className="grid gap-3 rounded-md border border-border p-3">
    <CommandEditor commands={task.validationCommands} label={t("plans.review.validationCommands")} onChange={(validationCommands) => onChange({ validationCommands })} prefix={`verify-${task.id}`} />
    <fieldset className="grid gap-2"><legend className="text-xs font-semibold">{t("plans.review.evidenceBindings")}</legend>{task.acceptanceCriteria.map((criterion, index) => {
      const binding = task.criterionEvidence.find((item) => item.criterionIndex === index) ?? { criterionIndex: index, kind: "manual" as const, commandId: null };
      return <div className="grid gap-2 sm:grid-cols-[1fr_9rem_12rem] sm:items-center" key={`${index}-${criterion}`}><span className="truncate text-xs text-muted-foreground">{criterion}</span><select aria-label={t("plans.review.evidenceKind", { number: index + 1 })} className={fieldClass} onChange={(event) => updateBinding(index, { kind: event.target.value === "automated" ? "automated" : "manual", commandId: event.target.value === "manual" ? null : task.validationCommands.find((command) => command.required)?.id ?? null })} value={binding.kind}><option value="automated">{t("plans.review.automated")}</option><option value="manual">{t("plans.review.manual")}</option></select><select aria-label={t("plans.review.evidenceCommand", { number: index + 1 })} className={fieldClass} disabled={binding.kind === "manual"} onChange={(event) => updateBinding(index, { commandId: event.target.value || null })} value={binding.commandId ?? ""}><option value="">—</option>{task.validationCommands.filter((command) => command.required).map((command) => <option key={command.id} value={command.id}>{command.id}</option>)}</select></div>;
    })}</fieldset>
  </div>;
}

function CommandEditor({ commands, label, onChange, prefix }: { commands: PlanVerificationCommand[]; label: string; onChange: (commands: PlanVerificationCommand[]) => void; prefix: string }) {
  const { t } = useTranslation();
  const update = (index: number, change: Partial<PlanVerificationCommand>) => onChange(commands.map((command, current) => current === index ? { ...command, ...change } : command));
  return <fieldset className="grid gap-2"><div className="flex items-center justify-between gap-2"><legend className="text-xs font-semibold">{label}</legend><Button onClick={() => onChange([...commands, newCommand(prefix)])} size="sm" type="button" variant="outline"><Plus aria-hidden="true" />{t("plans.review.addCommand")}</Button></div>{commands.map((command, index) => <div className="grid gap-2 rounded-md border border-border p-2 sm:grid-cols-[1fr_1fr_1.5fr_auto_auto] sm:items-center" key={`${command.id}-${index}`}><input aria-label={t("plans.review.commandId")} className={fieldClass} onChange={(event) => update(index, { id: event.target.value })} value={command.id} /><input aria-label={t("plans.review.commandProgram")} className={fieldClass} onChange={(event) => update(index, { program: event.target.value })} value={command.program} /><input aria-label={t("plans.review.commandArgs")} className={fieldClass} onChange={(event) => update(index, { args: event.target.value.split(" ").filter(Boolean) })} value={command.args.join(" ")} /><label className="flex items-center gap-1 text-xs"><input checked={command.required} onChange={(event) => update(index, { required: event.target.checked })} type="checkbox" />{t("plans.review.required")}</label><Button aria-label={t("plans.review.removeCommand")} onClick={() => onChange(commands.filter((_, current) => current !== index))} size="icon" type="button" variant="ghost"><Trash2 aria-hidden="true" /></Button></div>)}</fieldset>;
}
