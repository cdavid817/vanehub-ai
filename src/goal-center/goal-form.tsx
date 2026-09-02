import { type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { MutationStatus } from "../ui/async/MutationStatus";
import type { MutationState } from "../ui/async/mutation-state";
import type { Goal, GoalInput } from "../contracts/goal";

const fieldClass = "ucd-input rounded-md px-3 py-2 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

export function GoalForm({ goal, mutation, onCancel, onSubmit, submitLabel }: {
  goal?: Goal;
  /** This form's own in-flight create/update, if any -- drives the submit button's disabled
   *  state and an inline pending/error status, replacing the old page-wide `busy` boolean.
   *  Matches work-board-form.tsx's `WorkItemForm`. */
  mutation?: MutationState;
  onCancel: () => void;
  onSubmit: (input: GoalInput) => void;
  submitLabel: string;
}) {
  const { t } = useTranslation();
  const busy = mutation?.pending ?? false;

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    onSubmit({
      title: String(data.get("title") ?? ""),
      description: String(data.get("description") ?? ""),
      acceptanceNotes: String(data.get("acceptanceNotes") ?? ""),
      projectPath: String(data.get("projectPath") ?? "").trim() || null,
    });
  };

  return <form className="grid gap-2 rounded-md border border-border bg-muted/10 p-3" onSubmit={submit}>
    <input aria-label={t("goals.fields.title")} className={fieldClass} defaultValue={goal?.title} name="title" placeholder={t("goals.fields.title")} required />
    <input aria-label={t("goals.fields.projectPath")} className={fieldClass} defaultValue={goal?.projectPath ?? ""} name="projectPath" placeholder={t("goals.fields.projectPath")} />
    <textarea aria-label={t("goals.fields.description")} className={`${fieldClass} min-h-16`} defaultValue={goal?.description} name="description" placeholder={t("goals.fields.description")} />
    {/* Notes are read by whoever accepts the goal; nothing machine-checks them. */}
    <textarea aria-label={t("goals.fields.acceptanceNotes")} className={`${fieldClass} min-h-16`} defaultValue={goal?.acceptanceNotes} name="acceptanceNotes" placeholder={t("goals.fields.acceptanceNotes")} />
    <div className="flex items-center justify-end gap-2">
      <MutationStatus className="mr-auto" state={mutation} />
      <Button onClick={onCancel} type="button" variant="outline">{t("goals.actions.cancel")}</Button>
      <Button disabled={busy} type="submit">{submitLabel}</Button>
    </div>
  </form>;
}
