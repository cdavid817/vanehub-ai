import { Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";

export function WaitingIndicator() {
  const { t } = useTranslation();

  return (
    <span className="inline-flex items-center gap-2 text-xs text-muted-foreground">
      <Loader2 className="h-3.5 w-3.5 animate-spin" aria-hidden="true" />
      {t("chat.waiting")}
    </span>
  );
}
