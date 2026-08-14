import { useCallback, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "./application-dialog";
import { Button } from "./button";

export interface ConfirmationRequest {
  title: string;
  description?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  tone?: "default" | "danger";
}

/**
 * Promise-shaped replacement for `window.confirm`, which cannot be themed or localized and
 * renders as a browser chrome dialog inside the Tauri WebView.
 *
 * Callers keep their original control flow — `if (!(await confirm({...}))) return;` — and render
 * the returned node once anywhere in their tree.
 */
export function useConfirmation() {
  const { t } = useTranslation();
  const [request, setRequest] = useState<ConfirmationRequest | null>(null);
  const resolveRef = useRef<((confirmed: boolean) => void) | null>(null);

  const confirm = useCallback(
    (next: ConfirmationRequest) => new Promise<boolean>((resolve) => {
      // A second request while one is open cancels the first rather than stranding its promise.
      resolveRef.current?.(false);
      resolveRef.current = resolve;
      setRequest(next);
    }),
    [],
  );

  const settle = useCallback((confirmed: boolean) => {
    setRequest(null);
    const resolveRequest = resolveRef.current;
    resolveRef.current = null;
    resolveRequest?.(confirmed);
  }, []);

  const confirmationDialog: ReactNode = request ? (
    <ApplicationDialog
      footer={(
        <div className="flex justify-end gap-2">
          <Button onClick={() => settle(false)} size="sm" type="button" variant="outline">
            {request.cancelLabel ?? t("confirmation.cancel")}
          </Button>
          <Button
            className={request.tone === "danger" ? "bg-destructive text-destructive-foreground" : undefined}
            data-dialog-autofocus
            onClick={() => settle(true)}
            size="sm"
            type="button"
          >
            {request.confirmLabel ?? t("confirmation.confirm")}
          </Button>
        </div>
      )}
      maxWidth="max-w-sm"
      onClose={() => settle(false)}
      title={request.title}
    >
      {request.description ? (
        <p className="wrap-break-word text-sm leading-6 text-muted-foreground">{request.description}</p>
      ) : null}
    </ApplicationDialog>
  ) : null;

  return { confirm, confirmationDialog };
}
