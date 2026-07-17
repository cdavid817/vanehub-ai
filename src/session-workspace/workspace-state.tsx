import { AlertTriangle, LoaderCircle } from "lucide-react";
import { useTranslation } from "react-i18next";

export function WorkspaceState({
  kind,
  message,
}: {
  kind: "loading" | "empty" | "unavailable" | "error";
  message?: string;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex h-full min-h-40 flex-col items-center justify-center gap-2 rounded-lg border border-dashed border-border p-6 text-center text-sm text-muted-foreground">
      {kind === "loading" ? (
        <LoaderCircle aria-hidden="true" className="h-5 w-5 animate-spin text-primary" />
      ) : (
        <AlertTriangle aria-hidden="true" className="h-5 w-5 text-primary" />
      )}
      <p>{message ?? t(`sessionTabs.state.${kind}`)}</p>
    </div>
  );
}

export function PartialNotice() {
  const { t } = useTranslation();
  return <p className="rounded border border-border bg-muted px-2 py-1 text-xs text-muted-foreground">{t("sessionTabs.state.partial")}</p>;
}

