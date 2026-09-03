import { ChevronDown } from "lucide-react";
import { type FormEvent, useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { linkableGoalTargets, type GoalLinkTarget } from "../contracts/goal";

const fieldClass = "ucd-input rounded-md px-3 py-2 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

export interface ExecutionTargetRawIdFieldProps {
  pending: boolean;
  onLink: (targetKind: GoalLinkTarget, targetId: string) => void;
}

/**
 * 15.5's own "diagnostic raw-id path, explicitly advanced and validated": collapsed behind a
 * disclosure so `ExecutionTargetPicker`'s search is the ordinary path, for the case its search
 * cannot find a target -- e.g. recording a stale/deleted id for tracking (goal-management.md's
 * "when a linked object is deleted" case). `required` on the id input is the validation: an empty
 * submission never reaches `onLink`, same guarantee the pre-existing bare-form path already had.
 *
 * Keeps the exact same field names/labels the old always-visible form used
 * (`goals.fields.targetKind`/`goals.fields.targetId`) so this remains reachable and testable the
 * same way, just one disclosure click further in.
 */
export function ExecutionTargetRawIdField({ onLink, pending }: ExecutionTargetRawIdFieldProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const contentId = useId();

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    const targetId = String(data.get("targetId") ?? "").trim();
    if (!targetId) return;
    onLink(String(data.get("targetKind") ?? "loop") as GoalLinkTarget, targetId);
    event.currentTarget.reset();
    setOpen(false);
  }

  return (
    <div className="border-t border-border/60 pt-2">
      <button
        aria-controls={contentId}
        aria-expanded={open}
        className="flex items-center gap-1 text-xs font-medium text-muted-foreground hover:text-foreground"
        onClick={() => setOpen((current) => !current)}
        type="button"
      >
        <ChevronDown aria-hidden="true" className={`h-3.5 w-3.5 shrink-0 transition-transform ${open ? "rotate-180" : ""}`} />
        {t("goals.picker.advancedToggle")}
      </button>
      {open ? (
        <form className="mt-2 grid gap-2" id={contentId} onSubmit={submit}>
          <p className="text-xs text-muted-foreground">{t("goals.picker.advancedHint")}</p>
          <div className="flex flex-wrap gap-2">
            <select aria-label={t("goals.fields.targetKind")} className={fieldClass} defaultValue="loop" name="targetKind">
              {linkableGoalTargets.map((kind) => <option key={kind} value={kind}>{t(`goals.target.${kind}`)}</option>)}
            </select>
            <input aria-label={t("goals.fields.targetId")} className={`${fieldClass} min-w-0 flex-1`} name="targetId" placeholder={t("goals.fields.targetId")} required />
            <Button disabled={pending} size="sm" type="submit">{t("goals.actions.link")}</Button>
          </div>
        </form>
      ) : null}
    </div>
  );
}
