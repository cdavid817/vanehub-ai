import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { InstructionDraft, InstructionValues } from "./instruction-drafts";

function Side({
  label,
  revision,
  testId,
  values,
}: {
  label: string;
  revision: number;
  testId: string;
  values: InstructionValues;
}) {
  const { t } = useTranslation();
  return (
    <div className="min-w-0 rounded-md border border-border/70 p-3" data-testid={testId}>
      <div className="mb-2 flex flex-wrap items-baseline gap-2">
        <span className="text-sm font-medium">{label}</span>
        <span className="text-xs text-muted-foreground">
          {t("personalization.inheritance.revision", { revision })}
        </span>
        <span className="text-xs text-muted-foreground">
          {t(`personalization.editor.merge.${values.instructionMergeMode}`)}
        </span>
      </div>
      <div className="grid gap-2">
        {(["aboutUser", "styleRules"] as const).map((field) => (
          <div key={field}>
            <div className="text-xs font-medium text-muted-foreground">
              {t(`personalization.editor.${field}`)}
            </div>
            <p className="wrap-break-word whitespace-pre-wrap text-sm">
              {values[field] || t("personalization.inheritance.emptyField")}
            </p>
          </div>
        ))}
      </div>
    </div>
  );
}

/**
 * Both versions, and a choice the user makes.
 *
 * Saving stays disabled until they choose. The alternative -- letting whichever response landed
 * last decide -- destroys work silently, and silently is the part that matters: the user has no
 * signal that anything happened, so they never think to check.
 *
 * Reload exists because the stored side is a snapshot taken when the save was refused, and a
 * conflict left open while someone else keeps editing is answered against text that has moved on.
 */
export function ConflictPanel({
  draft,
  onKeepMine,
  onReload,
  onTakeTheirs,
}: {
  draft: InstructionDraft;
  onKeepMine: () => void;
  onReload: () => void;
  onTakeTheirs: () => void;
}) {
  const { t } = useTranslation();
  if (!draft.conflict) return null;

  return (
    <div
      className="flex flex-col gap-3 rounded-md border p-4 ucd-status-warning"
      data-testid="personalization-conflict"
      role="alert"
    >
      <div>
        <h4 className="text-sm font-semibold">{t("personalization.conflict.title")}</h4>
        <p className="mt-1 text-sm">
          {t("personalization.conflict.description", {
            attempted: draft.conflict.attemptedRevision,
            stored: draft.conflict.storedRevision,
          })}
        </p>
      </div>

      <div className="grid gap-3 sm:grid-cols-2">
        <Side
          label={t("personalization.conflict.yours")}
          revision={draft.conflict.attemptedRevision}
          testId="personalization-conflict-mine"
          values={draft.values}
        />
        <Side
          label={t("personalization.conflict.stored")}
          revision={draft.conflict.storedRevision}
          testId="personalization-conflict-stored"
          values={draft.conflict.stored}
        />
      </div>

      <div className="flex flex-wrap gap-3">
        <Button data-testid="personalization-conflict-keep-mine" onClick={onKeepMine}>
          {t("personalization.conflict.keepMine")}
        </Button>
        <Button data-testid="personalization-conflict-take-theirs" onClick={onTakeTheirs} variant="outline">
          {t("personalization.conflict.takeTheirs")}
        </Button>
        <Button data-testid="personalization-conflict-reload" onClick={onReload} variant="ghost">
          {t("personalization.conflict.reload")}
        </Button>
      </div>
    </div>
  );
}
