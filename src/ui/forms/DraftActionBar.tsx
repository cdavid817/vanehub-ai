import { LoaderCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";
import type { DisplayableError } from "../async/async-view-state";

export interface DraftActionBarProps {
  /** Count of fields that differ from their saved value; the bar renders nothing at 0. */
  dirtyCount: number;
  pending?: boolean;
  /** True disables Save while leaving Discard available — e.g. a local validation error or a
   *  server-side conflict blocks saving without also blocking the user from discarding the draft. */
  saveDisabled?: boolean;
  error?: DisplayableError;
  onSave: () => void;
  onDiscard: () => void;
  className?: string;
}

/**
 * The "draft" save-mode surface from design.md Decision 17 — batches edits behind explicit Save/Discard.
 *
 * `sticky bottom-0` overlaps content on appearance; it does not reserve space for itself, since
 * sticky positioning can't push sibling layout. A consumer whose page can be dirtied at a short
 * viewport needs its own bottom padding (conditional on `dirtyCount > 0`, to avoid permanent empty
 * space) so the bar never sits over another control — see `cli-parameters-page.tsx` for the pattern.
 */
export function DraftActionBar({ dirtyCount, pending = false, saveDisabled = false, error, onSave, onDiscard, className }: DraftActionBarProps) {
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
        <button className="ucd-focus-ring rounded-sm text-sm font-medium text-muted-foreground hover:underline" disabled={pending} onClick={onDiscard} type="button">
          {t("workbenchUi.draftBar.discard")}
        </button>
        <button
          className="ucd-focus-ring inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground disabled:opacity-50"
          disabled={pending || saveDisabled}
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
