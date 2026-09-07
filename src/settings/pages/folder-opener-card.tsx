import { ArrowDown, ArrowUp } from "lucide-react";
import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { FolderOpenerIcon } from "../../components/folder-opener-icon";
import { normalizeDisplayPath } from "../../lib/session-path";
import type { FolderOpenerAvailability, FolderOpenerStatus } from "../../types/folder-opener";
import { StatusPill } from "./page-parts";

const statusTone: Record<FolderOpenerStatus, "success" | "warning" | "danger" | "muted"> = {
  available: "success",
  "not-installed": "muted",
  "unsupported-platform": "muted",
  "invalid-installation": "warning",
  "detection-failed": "danger",
};

/**
 * The checkbox and its descriptive text share one `<label>`; the reorder buttons sit outside it,
 * because a label may not contain other interactive content and a screen reader would otherwise
 * announce the arrows as part of the checkbox.
 */
export function FolderOpenerCard({
  opener,
  checked,
  locked,
  busy,
  canMoveUp,
  canMoveDown,
  onToggle,
  onMove,
}: {
  opener: FolderOpenerAvailability;
  checked: boolean;
  locked: boolean;
  busy: boolean;
  canMoveUp: boolean;
  canMoveDown: boolean;
  onToggle: (enabled: boolean) => void;
  onMove: (direction: -1 | 1) => void;
}) {
  const { t } = useTranslation();
  const inputId = useId();
  const name = t(`folderOpeners.name.${opener.id}`);
  const details = [
    opener.version ? `${t("folderOpeners.version")}: ${opener.version}` : null,
    opener.edition ? `${t("folderOpeners.edition")}: ${opener.edition}` : null,
  ].filter((value): value is string => value !== null);

  return (
    <div className="flex items-start gap-3 rounded-lg border border-border bg-background p-3">
      <input
        checked={checked}
        className="mt-2"
        disabled={busy || locked}
        id={inputId}
        onChange={(event) => onToggle(event.target.checked)}
        type="checkbox"
      />
      <label className="flex min-w-0 flex-1 cursor-pointer items-start gap-3" htmlFor={inputId}>
        <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded border border-border bg-[hsl(var(--panel-muted))] text-primary">
          <FolderOpenerIcon id={opener.id} />
        </span>
        <span className="min-w-0 flex-1">
          <span className="flex flex-wrap items-center gap-2">
            <span className="text-sm font-medium text-foreground">{name}</span>
            <StatusPill status={t(`folderOpeners.status.${opener.status}`)} tone={statusTone[opener.status]} />
            {locked ? <span className="text-[11px] text-muted-foreground">{t("folderOpeners.fallback")}</span> : null}
          </span>
          {details.length > 0 ? <span className="mt-1 block text-[11px] text-muted-foreground">{details.join(" · ")}</span> : null}
          {opener.executablePath ? (
            <span className="mt-1 block truncate font-mono text-[11px] text-muted-foreground" title={normalizeDisplayPath(opener.executablePath)}>
              {normalizeDisplayPath(opener.executablePath)}
            </span>
          ) : null}
        </span>
      </label>
      {checked ? (
        <span className="flex shrink-0 gap-1">
          <Button className="h-7 w-7 px-0" disabled={busy || !canMoveUp} onClick={() => onMove(-1)} title={t("folderOpeners.moveUp")} type="button" variant="outline">
            <ArrowUp className="h-3.5 w-3.5" aria-hidden="true" />
            <span className="sr-only">{t("folderOpeners.moveUp")}</span>
          </Button>
          <Button className="h-7 w-7 px-0" disabled={busy || !canMoveDown} onClick={() => onMove(1)} title={t("folderOpeners.moveDown")} type="button" variant="outline">
            <ArrowDown className="h-3.5 w-3.5" aria-hidden="true" />
            <span className="sr-only">{t("folderOpeners.moveDown")}</span>
          </Button>
        </span>
      ) : null}
    </div>
  );
}
