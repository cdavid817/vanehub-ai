import { LoaderCircle, RefreshCcwDot } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useReducedMotion } from "../../hooks/use-reduced-motion";
import { cn } from "../../lib/utils";

export interface RefreshIndicatorProps {
  refreshing: boolean;
  stale?: boolean;
  className?: string;
}

/**
 * Status readout only — never blocks the content it decorates. Callers that want a manual
 * refresh trigger render their own button; this primitive only reports the current state.
 */
export function RefreshIndicator({ refreshing, stale = false, className }: RefreshIndicatorProps) {
  const { t } = useTranslation();
  const reducedMotion = useReducedMotion();
  if (!refreshing && !stale) return null;

  return (
    <span
      aria-live="polite"
      className={cn("inline-flex items-center gap-1.5 text-xs text-muted-foreground", className)}
      role="status"
    >
      {refreshing ? (
        <>
          <LoaderCircle aria-hidden="true" className={cn("h-3.5 w-3.5", !reducedMotion && "animate-spin")} />
          {t("workbenchUi.async.refreshing")}
        </>
      ) : (
        <>
          <RefreshCcwDot aria-hidden="true" className="h-3.5 w-3.5" />
          {t("workbenchUi.async.stale")}
        </>
      )}
    </span>
  );
}
