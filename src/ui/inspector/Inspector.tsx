import type { ReactNode } from "react";
import { Pin, PinOff, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { AsyncBoundary } from "../async/AsyncBoundary";
import type { AsyncViewState } from "../async/async-view-state";
import { cn } from "../../lib/utils";

export type InspectorMode = "overview" | "follow" | "pinned";

export interface InspectorProps {
  mode: InspectorMode;
  title: string;
  /** Caller-localized caption under the title describing the current selection, if any. */
  selectionSummary?: string;
  onPin: () => void;
  onUnpin: () => void;
  onReturnToOverview: () => void;
  /** Present when Inspector is hosted in a `Sheet` at narrower layout tiers. */
  onClose?: () => void;
  /** Shown when `mode === "overview"` — no selection to describe yet. */
  overview: ReactNode;
  /** Shown (via `AsyncBoundary`) for `follow`/`pinned` — the selected object's own detail. */
  detail: AsyncViewState<ReactNode>;
  onRetryDetail?: () => void;
  className?: string;
}

/**
 * The selection-driven shell from design.md Decision 8. Detail content is lazy-loaded and
 * rendered per selection `kind` by the caller — Inspector never copies a full editor, Diff, or
 * log in here; that content stays behind `EvidenceLink` on its authoritative page.
 */
export function Inspector({
  mode,
  title,
  selectionSummary,
  onPin,
  onUnpin,
  onReturnToOverview,
  onClose,
  overview,
  detail,
  onRetryDetail,
  className,
}: InspectorProps) {
  const { t } = useTranslation();

  return (
    <div className={cn("flex h-full min-h-0 flex-col", className)} data-testid="workbench-inspector">
      <div className="flex shrink-0 items-center justify-between gap-2 border-b border-border-subtle px-4 py-3">
        <div className="min-w-0">
          <h2 className="truncate text-sm font-semibold">{title}</h2>
          {selectionSummary ? <p className="truncate text-xs text-muted-foreground">{selectionSummary}</p> : null}
        </div>
        <div className="flex shrink-0 items-center gap-1">
          {mode === "follow" ? (
            <button aria-label={t("workbenchUi.inspector.pin")} className="ucd-focus-ring rounded-md p-1.5 hover:bg-accent" onClick={onPin} type="button">
              <Pin aria-hidden="true" className="h-4 w-4" />
            </button>
          ) : null}
          {mode === "pinned" ? (
            <button aria-label={t("workbenchUi.inspector.unpin")} className="ucd-focus-ring rounded-md p-1.5 hover:bg-accent" onClick={onUnpin} type="button">
              <PinOff aria-hidden="true" className="h-4 w-4" />
            </button>
          ) : null}
          {onClose ? (
            <button aria-label={t("workbenchUi.inspector.close")} className="ucd-focus-ring rounded-md p-1.5 hover:bg-accent" onClick={onClose} type="button">
              <X aria-hidden="true" className="h-4 w-4" />
            </button>
          ) : null}
        </div>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto p-4">
        {mode === "overview" ? overview : (
          <AsyncBoundary
            onRetry={onRetryDetail}
            state={detail}
            unavailableState={{
              title: t("workbenchUi.evidence.unavailable"),
              action: (
                <button className="ucd-focus-ring rounded-sm text-sm font-medium text-primary hover:underline" onClick={onReturnToOverview} type="button">
                  {t("workbenchUi.inspector.returnToOverview")}
                </button>
              ),
            }}
          >
            {(content) => content}
          </AsyncBoundary>
        )}
      </div>
    </div>
  );
}
