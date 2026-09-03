import { LoaderCircle, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";
import type { MutationState } from "./mutation-state";

export interface MutationStatusProps {
  state: MutationState | undefined;
  onRetry?: () => void;
  onDismiss?: () => void;
  className?: string;
}

/** Renders one target's mutation outcome inline, next to the action it came from. */
export function MutationStatus({ state, onRetry, onDismiss, className }: MutationStatusProps) {
  const { t } = useTranslation();
  if (!state) return null;

  if (state.pending) {
    return (
      <span className={cn("inline-flex items-center gap-1.5 text-xs text-muted-foreground", className)} role="status">
        <LoaderCircle aria-hidden="true" className="h-3.5 w-3.5 animate-spin" />
        {t("workbenchUi.mutation.pending")}
      </span>
    );
  }

  if (state.error) {
    const canRetry = state.error.retryable && Boolean(onRetry);
    return (
      <span className={cn("inline-flex items-center gap-2 text-xs text-destructive", className)} role="alert">
        {state.error.message}
        {canRetry ? (
          <button className="ucd-focus-ring rounded-sm font-medium underline-offset-4 hover:underline" onClick={onRetry} type="button">
            {t("featureLoad.retry")}
          </button>
        ) : null}
        {onDismiss ? (
          <button aria-label={t("workbenchUi.mutation.dismiss")} className="ucd-focus-ring rounded-sm" onClick={onDismiss} type="button">
            <X aria-hidden="true" className="h-3.5 w-3.5" />
          </button>
        ) : null}
      </span>
    );
  }

  return null;
}
