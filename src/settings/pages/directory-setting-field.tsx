import { FolderSearch } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { normalizeDisplayPath } from "../../lib/session-path";
import { cn } from "../../lib/utils";

const savedHintDurationMs = 2000;

/**
 * One text field for a directory setting: commits on Enter or blur, offers the native picker when
 * the runtime has one, and shows a short "saved" acknowledgement so a silent commit is not
 * mistaken for a no-op.
 *
 * Drafts and comparisons both use the display form of the stored path, so a stored value with a
 * Windows extended-length prefix is not re-saved on every blur just because it displays without it.
 */
export function DirectorySettingField({
  ariaLabel,
  canBrowse,
  className,
  disabled,
  onPick,
  onSave,
  placeholder,
  value,
}: {
  ariaLabel: string;
  canBrowse: boolean;
  className?: string;
  disabled: boolean;
  onPick: () => Promise<string | null>;
  onSave: (value: string) => Promise<void>;
  placeholder: string;
  value: string;
}) {
  const { t } = useTranslation();
  const stored = normalizeDisplayPath(value);
  const [draft, setDraft] = useState(stored);
  // What was last accepted, tracked locally so the blur that follows an Enter commit does not
  // save the same value again while the parent is still propagating the new stored value.
  const [committed, setCommitted] = useState(stored);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const savedTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    setDraft(stored);
    setCommitted(stored);
  }, [stored]);

  useEffect(() => () => {
    if (savedTimer.current) clearTimeout(savedTimer.current);
  }, []);

  function showSaved() {
    setSaved(true);
    if (savedTimer.current) clearTimeout(savedTimer.current);
    savedTimer.current = setTimeout(() => setSaved(false), savedHintDurationMs);
  }

  async function commit(next: string) {
    const trimmed = next.trim();
    if (trimmed === committed) return;
    setError(null);
    try {
      await onSave(trimmed);
      setCommitted(trimmed);
      showSaved();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function browse() {
    setError(null);
    try {
      const picked = await onPick();
      if (picked === null) return;
      setDraft(normalizeDisplayPath(picked));
      await commit(picked);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  return (
    <div className={cn("grid gap-1.5", className)}>
      <div className="flex gap-2">
        <input
          aria-label={ariaLabel}
          className="ucd-input h-9 min-w-0 flex-1 rounded-lg px-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
          disabled={disabled}
          onBlur={() => void commit(draft)}
          onChange={(event) => {
            setError(null);
            setDraft(event.target.value);
          }}
          onKeyDown={(event) => {
            if (event.key !== "Enter") return;
            event.preventDefault();
            void commit(draft);
          }}
          placeholder={placeholder}
          value={draft}
        />
        {canBrowse ? (
          <Button className="h-9 shrink-0" disabled={disabled} onClick={() => void browse()} title={t("basic.browseDirectory")} type="button" variant="outline">
            <FolderSearch className="h-4 w-4" aria-hidden="true" />
            <span className="sr-only sm:not-sr-only">{t("basic.browseDirectory")}</span>
          </Button>
        ) : null}
      </div>
      {/* Reserved line so the hint appearing does not shift the row below. */}
      <div aria-live="polite" className="min-h-4 text-xs leading-4">
        {error ? <span className="ucd-status-danger rounded border px-1.5 py-0.5">{error}</span> : null}
        {!error && saved ? <span className="text-[hsl(var(--success))]">{t("basic.saved")}</span> : null}
      </div>
    </div>
  );
}
