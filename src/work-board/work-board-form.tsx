import { type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { WorkItem, WorkItemPriority } from "../types/work-board";
import { workItemPriorities } from "../types/work-board";

export const fieldClass = "ucd-input rounded-md px-3 py-2 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

export type WorkItemFormValues = {
  title: string;
  description: string;
  priority: WorkItemPriority;
  projectPath: string | null;
  dueAt: string | null;
};

export function WorkItemForm({ busy, item, onCancel, onSubmit, submitLabel }: {
  busy: boolean;
  item?: WorkItem;
  onCancel: () => void;
  onSubmit: (input: WorkItemFormValues) => void;
  submitLabel: string;
}) {
  const { t } = useTranslation();
  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    onSubmit({
      title: String(data.get("title") ?? ""),
      description: String(data.get("description") ?? ""),
      priority: String(data.get("priority") ?? "none") as WorkItemPriority,
      projectPath: String(data.get("project") ?? "").trim() || null,
      dueAt: String(data.get("due") ?? "").trim() || null,
    });
  };

  return (
    <form className="grid gap-2 rounded-md border border-border bg-muted/10 p-3 md:grid-cols-2" onSubmit={submit}>
      <input aria-label={t("todoBoard.fields.title")} className={fieldClass} defaultValue={item?.title} name="title" placeholder={t("todoBoard.fields.title")} required />
      <input aria-label={t("todoBoard.fields.project")} className={fieldClass} defaultValue={item?.projectPath ?? ""} name="project" placeholder={t("todoBoard.fields.project")} />
      <textarea aria-label={t("todoBoard.fields.description")} className={`${fieldClass} md:col-span-2`} defaultValue={item?.description} name="description" placeholder={t("todoBoard.fields.description")} />
      <select aria-label={t("todoBoard.fields.priority")} className={fieldClass} defaultValue={item?.priority ?? "none"} name="priority">
        {workItemPriorities.map((value) => <option key={value} value={value}>{t(`todoBoard.priority.${value}`)}</option>)}
      </select>
      <input aria-label={t("todoBoard.fields.due")} className={fieldClass} defaultValue={item?.dueAt?.slice(0, 10) ?? ""} name="due" type="date" />
      <div className="flex justify-end gap-2 md:col-span-2">
        <Button onClick={onCancel} type="button" variant="outline">{t("todoBoard.cancel")}</Button>
        <Button disabled={busy} type="submit">{submitLabel}</Button>
      </div>
    </form>
  );
}
