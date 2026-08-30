import { LoaderCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";
import type { DisplayableError } from "../async/async-view-state";

export interface DraftActionBarProps {
  /** Count of fields that differ from their saved value; the bar renders nothing at 0. */
  dirtyCount: number;
  pending?: boolean;
  error?: DisplayableError;
  onSave: () => void;
  onDiscard: () => void;
  className?: string;
}

/** The "draft" save-mode surface from design.md Decision 17 — batches edits behind explicit Save/Discard. */
export function DraftActionBar({ dirtyCount, pending = false, error, onSave, onDiscard, className }: DraftActionBarProps) {
  const { t } = useTranslation();
  if (dirtyCount === 0) return null;

  return (
    <div
      className={cn(
        "ucd-raised sticky bottom-0 flex flex-wrap items-center justify-between gap-3 border-t border-border px-5 py-3 sm:px-6",
        className,
      )}
      role="region"
    >
      <div className="min-w-0 text-sm">
        <span>{t("workbenchUi.draftBar.unsavedChanges", { count: dirtyCount })}</span>
        {error ? <span className="ml-2 text-destructive">{error.message}</span> : null}
      </div>
      <div className="flex shrink-0 items-center gap-2">
        <button className="text-sm font-medium text-muted-foreground hover:underline" disabled={pending} onClick={onDiscard} type="button">
          {t("workbenchUi.draftBar.discard")}
        </button>
        <button
          className="ucd-focus-ring inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground disabled:opacity-50"
          disabled={pending}
          onClick={onSave}
          type="button"
        >
          {pending ? <LoaderCircle aria-hidden="true" className="h-3.5 w-3.5 animate-spin" /> : null}
          {t("workbenchUi.draftBar.save")}
        </button>
      </div>
    </div>
  );
}
