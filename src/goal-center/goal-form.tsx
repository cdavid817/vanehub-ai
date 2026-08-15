import { type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { Goal, GoalInput } from "../contracts/goal";

const fieldClass = "ucd-input rounded-md px-3 py-2 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

export function GoalForm({ busy, goal, onCancel, onSubmit, submitLabel }: {
  busy: boolean;
  goal?: Goal;
  onCancel: () => void;
  onSubmit: (input: GoalInput) => void;
  submitLabel: string;
}) {
  const { t } = useTranslation();

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
    <div className="flex justify-end gap-2">
      <Button onClick={onCancel} type="button" variant="outline">{t("goals.actions.cancel")}</Button>
      <Button disabled={busy} type="submit">{submitLabel}</Button>
    </div>
  </form>;
}
