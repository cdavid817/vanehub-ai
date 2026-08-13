import { AlertTriangle } from "lucide-react";
import { useTranslation } from "react-i18next";

export function UsageLoadError({ error }: { error: unknown }) {
  const { t } = useTranslation();
  return (
    <div className="flex gap-2 rounded-lg border p-4 text-sm ucd-status-danger" role="alert">
      <AlertTriangle className="h-4 w-4 shrink-0" aria-hidden="true" />
      {t("usage.error", { message: error instanceof Error ? error.message : String(error) })}
    </div>
  );
}

export function UsageEmptyState() {
  const { t } = useTranslation();
  return (
    <div className="ucd-panel rounded-lg p-8 text-center text-sm text-muted-foreground" role="status">
      {t("usage.empty")}
    </div>
  );
}
