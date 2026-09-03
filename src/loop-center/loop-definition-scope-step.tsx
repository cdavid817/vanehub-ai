import { useTranslation } from "react-i18next";
import { FormSection } from "../ui/forms/FormSection";
import type { LoopBranchChoice, LoopProjectChoice } from "../types/loop";
import { Field, inputClass, type StepProps } from "./loop-definition-step-fields";

// `<option>` labels can only be plain text, so a discovered-but-simulated (Web mock) choice is
// distinguished the same textual-suffix way as an unavailable one is, not with the badge
// `loop-run-header.tsx`/`loop-preflight-dialog.tsx` render for the same `loops.simulated` string
// once a real selection has been made — 17.6 needs the distinction visible at pick time too, not
// only after picking.
function optionSuffix(choice: { available: boolean; simulated: boolean }, t: (key: string) => string): string {
  const parts = [
    choice.available ? null : t("loops.editor.unavailable"),
    choice.simulated ? t("loops.simulated") : null,
  ].filter((part): part is string => part !== null);
  return parts.length ? ` — ${parts.join(", ")}` : "";
}

export function ScopeStep({ branches, draft, loading, projects, setDraft }: StepProps & { branches: LoopBranchChoice[]; loading: boolean; projects: LoopProjectChoice[] }) {
  const { t } = useTranslation();
  const projectOptions = retainProjectChoice(projects, draft.projectPath);
  const branchOptions = retainBranchChoice(branches, draft.baseBranch);
  return (
    <FormSection title={t("loops.editor.step.scope")}>
      <div className="grid gap-4 sm:grid-cols-2">
        <Field label="loops.editor.field.name"><input className={inputClass} value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} /></Field>
        <Field label="loops.editor.field.project"><select className={inputClass} disabled={loading && projectOptions.length === 0} value={draft.projectPath} onChange={(event) => setDraft({ ...draft, projectPath: event.target.value })}><option value="">{t("loops.editor.selectProject")}</option>{projectOptions.map((project) => <option key={project.path} value={project.path}>{project.displayName}{optionSuffix(project, t)}</option>)}</select></Field>
        <Field label="loops.editor.field.branch"><select className={inputClass} disabled={!draft.projectPath || (loading && branchOptions.length === 0)} value={draft.baseBranch} onChange={(event) => setDraft({ ...draft, baseBranch: event.target.value })}><option value="">{t("loops.editor.selectBranch")}</option>{branchOptions.map((branch) => <option key={`${branch.kind}:${branch.name}`} value={branch.name}>{branch.name}{optionSuffix(branch, t)}</option>)}</select></Field>
        <label className="flex min-h-9 items-center gap-2 text-sm"><input checked={draft.enabled} className="h-4 w-4 accent-primary" onChange={(event) => setDraft({ ...draft, enabled: event.target.checked })} type="checkbox" /><span>{t("loops.editor.field.enabled")}</span></label>
        <Field className="sm:col-span-2" label="loops.editor.field.goal"><textarea className={`${inputClass} min-h-20 py-2`} value={draft.goal} onChange={(event) => setDraft({ ...draft, goal: event.target.value })} /></Field>
        <Field className="sm:col-span-2" label="loops.editor.field.acceptance"><textarea className={`${inputClass} min-h-24 py-2`} value={draft.acceptanceCriteria} onChange={(event) => setDraft({ ...draft, acceptanceCriteria: event.target.value })} /></Field>
        <Field label="loops.editor.field.allowedPaths"><textarea className={`${inputClass} min-h-20 py-2`} value={draft.allowedPaths} onChange={(event) => setDraft({ ...draft, allowedPaths: event.target.value })} /></Field>
        <Field label="loops.editor.field.protectedPaths"><textarea className={`${inputClass} min-h-20 py-2`} value={draft.protectedPaths} onChange={(event) => setDraft({ ...draft, protectedPaths: event.target.value })} /></Field>
      </div>
    </FormSection>
  );
}

function retainProjectChoice(choices: LoopProjectChoice[], value: string) {
  if (!value || choices.some((choice) => choice.path === value)) return choices;
  return [{ path: value, displayName: value, available: false, simulated: false }, ...choices];
}

function retainBranchChoice(choices: LoopBranchChoice[], value: string) {
  if (!value || choices.some((choice) => choice.name === value)) return choices;
  return [{ name: value, kind: "local" as const, available: false, simulated: false }, ...choices];
}
