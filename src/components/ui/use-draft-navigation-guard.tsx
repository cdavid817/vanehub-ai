import { useCallback, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "./application-dialog";
import { Button } from "./button";

export type DraftNavigationOutcome = "save" | "discard" | "stay";

export interface DraftNavigationRequest {
  title: string;
  description?: string;
  dirtyCount: number;
  /** False disables the Save option without hiding it -- the draft is not currently saveable
   *  (local validation error or a server-side conflict), but Discard/Stay remain real choices. */
  canSave: boolean;
}

/**
 * Three-way sibling to `useConfirmation` for "you have unsaved changes" navigation prompts
 * (spec.md "Settings unsaved-change protection"): Save-and-leave, Discard-and-leave, or Stay,
 * instead of a plain confirm/cancel. Same promise-shaped call pattern so callers keep their
 * existing control flow: `const outcome = await requestDecision({...}); if (outcome === "stay") return;`
 */
export function useDraftNavigationGuard() {
  const { t } = useTranslation();
  const [request, setRequest] = useState<DraftNavigationRequest | null>(null);
  const resolveRef = useRef<((outcome: DraftNavigationOutcome) => void) | null>(null);

  const requestDecision = useCallback(
    (next: DraftNavigationRequest) => new Promise<DraftNavigationOutcome>((resolve) => {
      // A second request while one is open resolves the first as "stay" -- the safest default,
      // since silently discarding a draft the caller never got an answer for would be worse.
      resolveRef.current?.("stay");
      resolveRef.current = resolve;
      setRequest(next);
    }),
    [],
  );

  const settle = useCallback((outcome: DraftNavigationOutcome) => {
    setRequest(null);
    const resolveRequest = resolveRef.current;
    resolveRef.current = null;
    resolveRequest?.(outcome);
  }, []);

  const navigationGuardDialog: ReactNode = request ? (
    <ApplicationDialog
      footer={(
        <div className="flex flex-wrap justify-end gap-2">
          <Button onClick={() => settle("stay")} size="sm" type="button" variant="outline">
            {t("draftNavigationGuard.stay")}
          </Button>
          <Button
            className="text-destructive"
            onClick={() => settle("discard")}
            size="sm"
            type="button"
            variant="outline"
          >
            {t("draftNavigationGuard.discard")}
          </Button>
          <Button data-dialog-autofocus disabled={!request.canSave} onClick={() => settle("save")} size="sm" type="button">
            {t("draftNavigationGuard.saveAndLeave")}
          </Button>
        </div>
      )}
      maxWidth="max-w-sm"
      onClose={() => settle("stay")}
      title={request.title}
    >
      <p className="wrap-break-word text-sm leading-6 text-muted-foreground">
        {request.description ?? t("draftNavigationGuard.description", { count: request.dirtyCount })}
      </p>
    </ApplicationDialog>
  ) : null;

  return { requestDecision, navigationGuardDialog };
}
